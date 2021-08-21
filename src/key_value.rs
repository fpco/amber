use serde::Serialize;

#[derive(Serialize)]
pub struct KeyValue<'a, K, V> {
    key: &'a K,
    value: &'a V,
}

impl<'a, K, V> From<(&'a K, &'a V)> for KeyValue<'a, K, V> {
    fn from((key, value): (&'a K, &'a V)) -> Self {
        KeyValue { key, value }
    }
}
