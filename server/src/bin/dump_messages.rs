//! Port stub of server-go-old/cmd/dump-messages/main.go.
//!
//! The Go tool dumped buffered Discord messages (used by the
//! cleanup-loop GitHub Action). It depends on the Discord bot, which
//! is not yet ported (see src/discord.rs). This binary exists so the
//! Cargo target mapping in server-go-old/README.md is real; it will be
//! filled in alongside the Discord port.
//!
//! TODO(port): server-go-old/cmd/dump-messages/main.go

fn main() {
    eprintln!(
        "dump-messages: not yet ported to Rust (depends on the Discord \
         bot port). See server-go-old/cmd/dump-messages/main.go and \
         todo.md. Exiting 0 so CI scripts don't hard-fail."
    );
}
