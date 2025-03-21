use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Step failed: `{0}`")]
    Step(StepError),
    #[error("Task failed: `{0}`")]
    Task(TaskError),
    #[error("State check failed: `{0}`")]
    StateCheck(CheckError),
}

#[derive(Error, Debug)]
pub enum StepError {
    #[error("Wallet failed: `{0}`")]
    Wallet(String),
    #[error("Building task failed: `{0}`")]
    BuildTask(String),
    #[error("Query failed: `{0}`")]
    Query(QueryError),
}

#[derive(Error, Debug)]
pub enum TaskError {
    #[error("Wallet failed: `{0}`")]
    Wallet(String),
    #[error("Building tx failed: `{0}`")]
    BuildTx(String),
    #[error("Building check failed: `{0}`")]
    BuildCheck(String),
    #[error("Broadcasting tx failed: `{0}`")]
    Broadcast(namada_sdk::error::Error),
    #[error("Executing tx failed: `{0}`")]
    Execution(String),
    #[error("Executing tx failed due to the gas: `{0}`")]
    InsufficientGas(String),
    #[error("Shielded tx failed due to crossing the epoch boundary: `{0}`")]
    InvalidShielded(String),
    #[error("Query failed: `{0}`")]
    Query(QueryError),
}

#[derive(Error, Debug)]
pub enum CheckError {
    #[error("Query failed: `{0}`")]
    Query(QueryError),
    #[error("State check failed: `{0}`")]
    State(String),
}

#[derive(Error, Debug)]
pub enum QueryError {
    #[error("Wallet failed: `{0}`")]
    Wallet(String),
    #[error("Namada RPC request failed `{0}`")]
    Rpc(namada_sdk::error::Error),
    #[error("Fetching shielded context data failed: `{0}`")]
    ShieldedSync(String),
    #[error("Shielded context failed: `{0}`")]
    ShieldedContext(String),
}

impl From<QueryError> for StepError {
    fn from(err: QueryError) -> Self {
        StepError::Query(err)
    }
}

impl From<QueryError> for TaskError {
    fn from(err: QueryError) -> Self {
        TaskError::Query(err)
    }
}

impl From<QueryError> for CheckError {
    fn from(err: QueryError) -> Self {
        CheckError::Query(err)
    }
}
