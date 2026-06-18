use std::fmt::{Display, Formatter};

use vsm_rs::protocol::system1::{CapacitySnapshot, UnitDescriptor};
use vsm_rs::protocol::RuntimeId;
use vsm_rs::roles::system1::testing::{AcceptAllWorkModel, StaticOperationalUnitFactory};
use vsm_rs::roles::ViableSystem;
use vsm_rs::VsmBuilder;

#[derive(Clone, Debug)]
struct ExampleWork;

#[derive(Clone, Debug)]
struct ExampleOutcome;

#[derive(Debug)]
struct ExampleError;

impl Display for ExampleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("example error")
    }
}

impl std::error::Error for ExampleError {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ExampleCapability(&'static str);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ExampleUnitId(&'static str);

struct ExampleSnapshot;

struct ExampleSystem;

impl ViableSystem for ExampleSystem {
    type Work = ExampleWork;
    type Outcome = ExampleOutcome;
    type AppError = ExampleError;
    type Capability = ExampleCapability;
    type UnitId = ExampleUnitId;
    type UnitSnapshot = ExampleSnapshot;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let descriptor =
        UnitDescriptor::<ExampleSystem>::new(ExampleUnitId("unit-a"), [ExampleCapability("work")]);
    let factory = StaticOperationalUnitFactory::new(
        descriptor,
        CapacitySnapshot::new(0, Some(4), 0.0),
        ExampleOutcome,
    );

    let runtime = VsmBuilder::new()
        .runtime_id(RuntimeId::from_string("example-runtime"))
        .work_model(AcceptAllWorkModel::new([ExampleCapability("work")]))
        .operational_unit_factory(factory)
        .start()
        .await?;

    assert!(runtime.is_ready());
    println!("runtime {} ready", runtime.runtime_id());

    runtime.shutdown().await?;
    Ok(())
}
