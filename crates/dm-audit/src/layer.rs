use crate::{AuditEvent, AuditLogger};
use serde_json::{Map, Value};
use tracing::{
    field::{Field, Visit},
    Event, Subscriber,
};
use tracing_subscriber::{layer::Context, Layer};

pub struct DiagnosticLayer(pub AuditLogger);
struct Fields(Map<String, Value>);
impl Visit for Fields {
    fn record_debug(&mut self, _: &Field, _: &dyn std::fmt::Debug) {} // Never persist arbitrary Debug values.
    fn record_i64(&mut self, f: &Field, v: i64) {
        self.0.insert(f.name().into(), v.into());
    }
    fn record_u64(&mut self, f: &Field, v: u64) {
        self.0.insert(f.name().into(), v.into());
    }
    fn record_bool(&mut self, f: &Field, v: bool) {
        self.0.insert(f.name().into(), v.into());
    }
    fn record_str(&mut self, f: &Field, v: &str) {
        if f.name() != "message" {
            self.0.insert(f.name().into(), v.into());
        }
    }
}
impl<S: Subscriber> Layer<S> for DiagnosticLayer {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        if event.metadata().target() != "dm_diagnostic" {
            return;
        }
        let mut fields = Fields(Map::new());
        event.record(&mut fields);
        let mut entry = AuditEvent::succeeded("runtime", "");
        entry.audience = "support".into();
        if *event.metadata().level() == tracing::Level::ERROR {
            entry.level = crate::AuditLevel::Error;
            entry.outcome = crate::AuditOutcome::Failed;
        }
        entry.details = Value::Object(fields.0);
        self.0.record_now(entry);
    }
}
