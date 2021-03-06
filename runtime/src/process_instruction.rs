use solana_sdk::{
    account::{Account, KeyedAccount},
    instruction::{CompiledInstruction, Instruction, InstructionError},
    message::Message,
    pubkey::Pubkey,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

// Prototype of a native loader entry point
///
/// program_id: Program ID of the currently executing program
/// keyed_accounts: Accounts passed as part of the instruction
/// instruction_data: Instruction data
/// invoke_context: Invocation context
pub type LoaderEntrypoint = unsafe extern "C" fn(
    program_id: &Pubkey,
    keyed_accounts: &[KeyedAccount],
    instruction_data: &[u8],
    invoke_context: &dyn InvokeContext,
) -> Result<(), InstructionError>;

pub type ProcessInstruction = fn(&Pubkey, &[KeyedAccount], &[u8]) -> Result<(), InstructionError>;
pub type ProcessInstructionWithContext =
    fn(&Pubkey, &[KeyedAccount], &[u8], &mut dyn InvokeContext) -> Result<(), InstructionError>;

// These are just type aliases for work around of Debug-ing above function pointers
pub type ErasedProcessInstructionWithContext = fn(
    &'static Pubkey,
    &'static [KeyedAccount<'static>],
    &'static [u8],
    &'static mut dyn InvokeContext,
) -> Result<(), InstructionError>;

pub type ErasedProcessInstruction = fn(
    &'static Pubkey,
    &'static [KeyedAccount<'static>],
    &'static [u8],
) -> Result<(), InstructionError>;

/// Invocation context passed to loaders
pub trait InvokeContext {
    /// Push a program ID on to the invocation stack
    fn push(&mut self, key: &Pubkey) -> Result<(), InstructionError>;
    /// Pop a program ID off of the invocation stack
    fn pop(&mut self);
    /// Verify and update PreAccount state based on program execution
    fn verify_and_update(
        &mut self,
        message: &Message,
        instruction: &CompiledInstruction,
        accounts: &[Rc<RefCell<Account>>],
    ) -> Result<(), InstructionError>;
    /// Get the program ID of the currently executing program
    fn get_caller(&self) -> Result<&Pubkey, InstructionError>;
    /// Get a list of built-in programs
    fn get_programs(&self) -> &[(Pubkey, ProcessInstruction)];
    /// Get this invocation's logger
    fn get_logger(&self) -> Rc<RefCell<dyn Logger>>;
    /// Are cross program invocations supported
    fn is_cross_program_supported(&self) -> bool;
    /// Get this invocation's compute budget
    fn get_compute_budget(&self) -> ComputeBudget;
    /// Get this invocation's compute meter
    fn get_compute_meter(&self) -> Rc<RefCell<dyn ComputeMeter>>;
    /// Loaders may need to do work in order to execute a program.  Cache
    /// the work that can be re-used across executions
    fn add_executor(&mut self, pubkey: &Pubkey, executor: Arc<dyn Executor>);
    /// Get the completed loader work that can be re-used across executions
    fn get_executor(&mut self, pubkey: &Pubkey) -> Option<Arc<dyn Executor>>;
    /// Record invoked instruction
    fn record_instruction(&self, instruction: &Instruction);
}

#[derive(Clone, Copy, Debug)]
pub struct ComputeBudget {
    /// Number of compute units that an instruction is allowed.  Compute units
    /// are consumed by program execution, resources they use, etc...
    pub max_units: u64,
    /// Number of compute units consumed by a log call
    pub log_units: u64,
    /// Number of compute units consumed by a log_u64 call
    pub log_64_units: u64,
    /// Number of compute units consumed by a create_program_address call
    pub create_program_address_units: u64,
    /// Number of compute units consumed by an invoke call (not including the cost incured by
    /// the called program)
    pub invoke_units: u64,
    /// Maximum cross-program invocation depth allowed including the orignal caller
    pub max_invoke_depth: usize,
}
impl Default for ComputeBudget {
    fn default() -> Self {
        // Tuned for ~1ms
        ComputeBudget {
            max_units: 200_000,
            log_units: 100,
            log_64_units: 100,
            create_program_address_units: 1500,
            invoke_units: 1000,
            max_invoke_depth: 2,
        }
    }
}

/// Compute meter
pub trait ComputeMeter {
    /// Consume compute units
    fn consume(&mut self, amount: u64) -> Result<(), InstructionError>;
    /// Get the number of remaining compute units
    fn get_remaining(&self) -> u64;
}

/// Log messages
pub trait Logger {
    fn log_enabled(&self) -> bool;
    /// Log a message
    fn log(&mut self, message: &str);
}

/// Program executor
pub trait Executor: Send + Sync {
    /// Execute the program
    fn execute(
        &self,
        program_id: &Pubkey,
        keyed_accounts: &[KeyedAccount],
        instruction_data: &[u8],
        invoke_context: &mut dyn InvokeContext,
    ) -> Result<(), InstructionError>;
}
