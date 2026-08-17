//! A minimal LSP client, spoken to rust-analyzer over stdio.
//!
//! Why an analyser and not a parser: the question this lens answers is "what
//! does this call", and in Rust that question needs type inference. `x.len()`
//! is `slice::len` or `Vec::len` or `HashMap::len` depending on what `x` is,
//! and a syntax tree cannot tell you which. rust-analyzer already knows,
//! already indexes the dependency sources, and is already in this project's
//! pinned toolchain — so the cost is a subprocess rather than a second
//! type system.
//!
//! Measured on `liquid-cache` (43k lines, 134 files): 8.4s to index, 0.3s for
//! every file's symbol tree, 1.1s for the call hierarchy of all 1546
//! functions. The indexing dominates, and everything after it is nearly free,
//! which is why this asks for the whole workspace at once rather than lazily.
//!
//! Only the five requests this lens needs are modelled. There is no `lsp-types`
//! dependency: the wire shapes used here are small, and hand-rolling them keeps
//! the client's dependency list honest about what it actually needs.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// How long to wait for rust-analyzer to finish indexing before giving up.
/// Generous, because the wait scales with the workspace and a timeout here
/// means no lens at all.
const INDEX_TIMEOUT: Duration = Duration::from_secs(600);
/// How long any single request may take.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// A position in a file, zero-based, as LSP counts them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// One symbol from `textDocument/documentSymbol`, with its children. This is
/// the unit tree, handed over by the analyser rather than reconstructed: an
/// `impl` block arrives already named `impl Display for Palette`, which is
/// where this lens's trait annotations come from.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    /// LSP `SymbolKind`. 2 Module, 5 Class, 6 Method, 10 Enum, 11 Interface
    /// (a trait), 12 Function, 19 Object (an impl block), 23 Struct.
    pub kind: u8,
    pub detail: Option<String>,
    /// Where the name itself sits.
    ///
    /// Deliberately the *name's* position and not the item's range: an item's
    /// range already includes its attributes, so anchoring test detection there
    /// starts the search below the `#[cfg(test)]` and finds nothing.
    pub selection: Position,
    pub children: Vec<Symbol>,
}

/// One end of a call, as `callHierarchy` reports it.
///
/// The file and the position, and nothing else: the target is matched back to a
/// unit already built from that file's symbol tree, so its name and signature
/// come from there rather than being carried twice and risking disagreement.
#[derive(Debug, Clone)]
pub struct CallTarget {
    pub uri: String,
    pub selection: Position,
}

/// A running rust-analyzer.
pub struct Analyzer {
    child: Child,
    stdin: Mutex<ChildStdin>,
    next_id: AtomicI64,
    pending: Arc<Mutex<HashMap<i64, Sender<Result<Value, String>>>>>,
    /// Set by the reader thread when the server reports it has stopped working.
    quiescent: Arc<Mutex<bool>>,
    root: PathBuf,
}

impl Analyzer {
    /// Start rust-analyzer against a workspace and wait for it to finish
    /// indexing. Returns once the server reports itself quiescent, which is the
    /// point at which call hierarchy answers are complete rather than partial —
    /// asking earlier gets silence for half the workspace.
    pub fn start(root: &Path) -> Result<Self, String> {
        let mut child = Command::new("rust-analyzer")
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // rust-analyzer is chatty on stderr and none of it is ours to show.
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                format!(
                    "could not start rust-analyzer: {e}. This lens reads the workspace through \
                     rust-analyzer, so it needs `rust-analyzer` on PATH — it ships with the \
                     `rust-analyzer` rustup component."
                )
            })?;

        let stdin = child.stdin.take().ok_or("rust-analyzer has no stdin")?;
        let stdout = child.stdout.take().ok_or("rust-analyzer has no stdout")?;

        let pending: Arc<Mutex<HashMap<i64, Sender<Result<Value, String>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let quiescent = Arc::new(Mutex::new(false));

        // One reader thread owns the pipe and hands every response to whoever
        // is waiting on that id. Requests can then be in flight together, which
        // is the whole reason the extraction takes a second rather than a
        // minute.
        {
            let pending = pending.clone();
            let quiescent = quiescent.clone();
            std::thread::spawn(move || read_loop(stdout, pending, quiescent));
        }

        let analyzer = Self {
            child,
            stdin: Mutex::new(stdin),
            next_id: AtomicI64::new(1),
            pending,
            quiescent,
            root: root.to_path_buf(),
        };

        let root_uri = uri_of(root);
        analyzer.request(
            "initialize",
            json!({
                "processId": std::process::id(),
                "rootUri": root_uri,
                "workspaceFolders": [{ "uri": root_uri, "name": "root" }],
                "capabilities": {
                    "window": { "workDoneProgress": true },
                    "textDocument": {
                        "documentSymbol": { "hierarchicalDocumentSymbolSupport": true },
                        "callHierarchy": { "dynamicRegistration": false },
                    },
                    // The one signal that says indexing is finished. Without it
                    // there is only progress-token guesswork.
                    "experimental": { "serverStatusNotification": true },
                },
                "initializationOptions": {
                    "cachePriming": { "enable": true },
                    "procMacro": { "enable": true },
                    // Nothing here needs diagnostics, and `cargo check` on a
                    // cold workspace costs more than everything else combined.
                    "checkOnSave": false,
                },
            }),
        )?;
        analyzer.notify("initialized", json!({}))?;
        analyzer.wait_until_indexed()?;
        Ok(analyzer)
    }

    fn wait_until_indexed(&self) -> Result<(), String> {
        let deadline = Instant::now() + INDEX_TIMEOUT;
        while Instant::now() < deadline {
            if *self.quiescent.lock().map_err(|_| "lock poisoned")? {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(120));
        }
        Err(format!(
            "rust-analyzer did not finish indexing {} within {}s",
            self.root.display(),
            INDEX_TIMEOUT.as_secs()
        ))
    }

    fn send(&self, message: &Value) -> Result<(), String> {
        let body = serde_json::to_vec(message).map_err(|e| e.to_string())?;
        let mut stdin = self.stdin.lock().map_err(|_| "lock poisoned")?;
        write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).map_err(|e| e.to_string())?;
        stdin.write_all(&body).map_err(|e| e.to_string())?;
        stdin.flush().map_err(|e| e.to_string())
    }

    pub fn notify(&self, method: &str, params: Value) -> Result<(), String> {
        self.send(&json!({ "jsonrpc": "2.0", "method": method, "params": params }))
    }

    /// Fire a request without waiting for it. The receiver is the claim ticket;
    /// holding several at once is what pipelines the extraction.
    pub fn start_request(
        &self,
        method: &str,
        params: Value,
    ) -> Result<Receiver<Result<Value, String>>, String> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = channel();
        self.pending
            .lock()
            .map_err(|_| "lock poisoned")?
            .insert(id, tx);
        self.send(&json!({
            "jsonrpc": "2.0", "id": id, "method": method, "params": params
        }))?;
        Ok(rx)
    }

    pub fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        let rx = self.start_request(method, params)?;
        collect(rx)
    }

    /// Tell the server about a file. Call hierarchy needs the document open;
    /// for dependency sources this is also what pulls them into the session.
    pub fn open(&self, path: &Path, text: &str) -> Result<(), String> {
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri_of(path),
                    "languageId": "rust",
                    "version": 1,
                    "text": text,
                }
            }),
        )
    }

    pub fn document_symbols(&self, path: &Path) -> Result<Vec<Symbol>, String> {
        let value = self.request(
            "textDocument/documentSymbol",
            json!({ "textDocument": { "uri": uri_of(path) } }),
        )?;
        Ok(parse_symbols(&value))
    }

    pub fn start_prepare(
        &self,
        uri: &str,
        at: Position,
    ) -> Result<Receiver<Result<Value, String>>, String> {
        self.start_request(
            "textDocument/prepareCallHierarchy",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": at.line, "character": at.character },
            }),
        )
    }

    pub fn start_outgoing(&self, item: &Value) -> Result<Receiver<Result<Value, String>>, String> {
        self.start_request("callHierarchy/outgoingCalls", json!({ "item": item }))
    }
}

impl Drop for Analyzer {
    fn drop(&mut self) {
        // A language server left running outlives the request that wanted it.
        let _ = self.notify("shutdown", json!(null));
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Wait on one in-flight request.
pub fn collect(rx: Receiver<Result<Value, String>>) -> Result<Value, String> {
    match rx.recv_timeout(REQUEST_TIMEOUT) {
        Ok(result) => result,
        Err(_) => Err("rust-analyzer stopped answering".to_string()),
    }
}

/// Outgoing calls, already reduced to what the extractor needs.
pub fn parse_outgoing(value: &Value) -> Vec<CallTarget> {
    value
        .as_array()
        .map(|calls| {
            calls
                .iter()
                .filter_map(|call| {
                    let to = call.get("to")?;
                    Some(CallTarget {
                        uri: to.get("uri")?.as_str()?.to_string(),
                        selection: position_of(to.get("selectionRange")?.get("start")?)
                            .unwrap_or(Position {
                                line: 0,
                                character: 0,
                            }),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_symbols(value: &Value) -> Vec<Symbol> {
    value
        .as_array()
        .map(|items| items.iter().filter_map(parse_symbol).collect())
        .unwrap_or_default()
}

fn parse_symbol(value: &Value) -> Option<Symbol> {
    Some(Symbol {
        name: value.get("name")?.as_str()?.to_string(),
        kind: value.get("kind")?.as_u64()? as u8,
        detail: value
            .get("detail")
            .and_then(Value::as_str)
            .map(str::to_string),
        selection: position_of(value.get("selectionRange")?.get("start")?)?,
        children: value
            .get("children")
            .map(parse_symbols)
            .unwrap_or_default(),
    })
}

fn position_of(value: &Value) -> Option<Position> {
    Some(Position {
        line: value.get("line")?.as_u64()? as u32,
        character: value.get("character")?.as_u64()? as u32,
    })
}

/// A `file://` URI, percent-encoding the characters that actually turn up in
/// store paths and crate names. Nix store paths and registry directories are
/// otherwise fine as-is.
pub fn uri_of(path: &Path) -> String {
    let text = path.to_string_lossy();
    let mut out = String::from("file://");
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' | b'+' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// The filesystem path a `file://` URI names.
pub fn path_of(uri: &str) -> PathBuf {
    let body = uri.strip_prefix("file://").unwrap_or(uri);
    let bytes = body.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(byte) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    PathBuf::from(String::from_utf8_lossy(&out).into_owned())
}

/// Read framed messages until the pipe closes, handing each response to its
/// waiting caller and watching for the one notification that says indexing is
/// done.
fn read_loop(
    stdout: impl Read,
    pending: Arc<Mutex<HashMap<i64, Sender<Result<Value, String>>>>>,
    quiescent: Arc<Mutex<bool>>,
) {
    let mut reader = BufReader::new(stdout);
    loop {
        // Headers, terminated by a blank line.
        let mut length: Option<usize> = None;
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => return,
                Ok(_) => {}
                Err(_) => return,
            }
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                break;
            }
            if let Some(value) = trimmed
                .strip_prefix("Content-Length:")
                .or_else(|| trimmed.strip_prefix("content-length:"))
            {
                length = value.trim().parse().ok();
            }
        }
        let Some(length) = length else { return };
        let mut body = vec![0u8; length];
        if reader.read_exact(&mut body).is_err() {
            return;
        }
        let Ok(message) = serde_json::from_slice::<Value>(&body) else {
            continue;
        };

        if let Some(id) = message.get("id").and_then(Value::as_i64) {
            // A request from the server to us carries a method too; those are
            // registrations and configuration reads we have no answer for.
            if message.get("method").is_some() {
                continue;
            }
            let sender = pending.lock().ok().and_then(|mut map| map.remove(&id));
            if let Some(sender) = sender {
                let result = match message.get("error") {
                    Some(error) => Err(error.to_string()),
                    None => Ok(message.get("result").cloned().unwrap_or(Value::Null)),
                };
                let _ = sender.send(result);
            }
        } else if message.get("method").and_then(Value::as_str) == Some("experimental/serverStatus")
            && message
                .get("params")
                .and_then(|p| p.get("quiescent"))
                .and_then(Value::as_bool)
                == Some(true)
            && let Ok(mut flag) = quiescent.lock()
        {
            *flag = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-tripping matters more than the exact encoding: every path this
    /// lens handles goes out as a URI and comes back as one, and a registry
    /// directory carrying a `+` in a version was what broke the naive version.
    #[test]
    fn uris_round_trip_through_paths() {
        for original in [
            "/home/user/code/my-crate/src/lib.rs",
            "/home/user/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/arrow-array-58.3.0/src/lib.rs",
            "/nix/store/abc-rust/lib/rustlib/src/rust/library/core/src/option.rs",
            "/home/user/a space/and'quote/x.rs",
            "/home/user/wasi-0.11.0+wasi-snapshot/src/lib.rs",
        ] {
            let path = PathBuf::from(original);
            assert_eq!(path_of(&uri_of(&path)), path, "{original} did not survive");
        }
    }

    #[test]
    fn a_uri_without_escapes_is_read_verbatim() {
        assert_eq!(
            path_of("file:///home/user/src/lib.rs"),
            PathBuf::from("/home/user/src/lib.rs")
        );
    }

    #[test]
    fn symbols_parse_with_their_children() {
        let value = serde_json::json!([{
            "name": "impl Display for Palette",
            "kind": 19,
            "range": { "start": { "line": 3, "character": 0 }, "end": { "line": 9, "character": 1 } },
            "selectionRange": { "start": { "line": 3, "character": 5 }, "end": { "line": 3, "character": 9 } },
            "children": [{
                "name": "fmt",
                "kind": 6,
                "detail": "fn(&self, f: &mut Formatter<'_>) -> fmt::Result",
                "range": { "start": { "line": 4, "character": 4 }, "end": { "line": 8, "character": 5 } },
                "selectionRange": { "start": { "line": 4, "character": 7 }, "end": { "line": 4, "character": 10 } },
            }],
        }]);
        let symbols = parse_symbols(&value);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "impl Display for Palette");
        assert_eq!(symbols[0].kind, 19);
        assert_eq!(symbols[0].children.len(), 1);
        assert_eq!(symbols[0].children[0].name, "fmt");
        // The name's own line, not the range's start.
        assert_eq!(symbols[0].children[0].selection.line, 4);
    }

    #[test]
    fn outgoing_calls_parse_to_targets() {
        let value = serde_json::json!([{
            "to": {
                "name": "push",
                "kind": 6,
                "detail": "pub fn push(&mut self, value: T)",
                "uri": "file:///lib/alloc/src/vec/mod.rs",
                "range": { "start": { "line": 1, "character": 0 }, "end": { "line": 2, "character": 0 } },
                "selectionRange": { "start": { "line": 1, "character": 11 }, "end": { "line": 1, "character": 15 } },
            },
            "fromRanges": [],
        }]);
        let targets = parse_outgoing(&value);
        assert_eq!(targets.len(), 1);
        assert!(targets[0].uri.ends_with("vec/mod.rs"));
        assert_eq!(targets[0].selection.line, 1);
        assert_eq!(targets[0].selection.character, 11);
    }
}
