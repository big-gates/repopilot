//! CLI 인터페이스 모듈 묶음.
//! 입력 파싱/REPL/UI/조립을 한 네임스페이스로 관리한다.

pub mod command;
pub mod composition;
pub mod repl;
pub mod repl_input;

pub use command::{Cli, CliAction};
pub use composition::AppComposition;
pub use repl::run_repl;
