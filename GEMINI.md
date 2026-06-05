# 🧠 Project Context & Core Directives
- **Stack**: Rust (Backend) + HTML/CSS/JS (Browser Frontend).
- **Goal**: Intermediate-level rewrite from TypeScript. The objective is native OS compatibility, minimal resource usage, and blazing-fast execution.
- **User Level**: Rust beginner. The code must be highly optimized but written cleanly enough for a learner to fully understand.

# 🛑 ABSOLUTE RULES (CRITICAL - DO NOT IGNORE)
1. **NO DELETED CODE**: NEVER delete, remove, or refactor existing working code or files unless explicitly given permission. 
2. **CLARIFY BEFORE CODING**: If a feature request is ambiguous, STOP. Ask specific clarifying questions first. Do not guess or hallucinate requirements.
3. **NO BACK-AND-FORTH FAILURES**: Once requirements are clear, provide the exact, working feature as described in a single, complete response. 

# 📜 Architecture & Coding Standards
- **Modularity**: I strictly prefer many small, single-purpose files over a few monolithic, huge files. Keep the file tree organized.
- **Simplicity & Readability**: Write clean, idiomatic Rust. Avoid overly clever abstractions if a simpler, readable approach exists. 
- **Maximum Optimization**: Zero memory leaks. Zero bottlenecks. Take full advantage of multi-threading and multi-core processing (e.g., using `std::thread`, `tokio`, or `rayon` where appropriate) for heavy tasks.
- **Clear Boundaries**: Keep the Rust backend logic strictly decoupled from the HTML/CSS/JS frontend.

# 🤖 Operational Protocols & Verification
1. **Atomic Actions**: Focus on one specific feature or fix per prompt. 
2. **Mandatory Pre-Flight Checks**: You MUST use `cargo check` to verify syntax, types, and borrow-checker rules before presenting any final code. Do not hand me code that fails to compile.
3. **Release Ready**: All code provided must be able to compile successfully under `cargo build --release`.
4. **Educational but Concise**: Provide the working code immediately. Only explain the logic if it involves complex Rust concepts (like lifetimes, traits, or concurrency) so I can learn, but keep explanations direct and fluff-free.