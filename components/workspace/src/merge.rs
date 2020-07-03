use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Error;

pub fn map<T>(base: &T, from: &T) -> Result<T, Error>
where
    T: Serialize + DeserializeOwned,
{
    let base_val = serde_json::to_value(base)?;
    let from_val = serde_json::to_value(from)?;

    let mut base_map = base_val.as_object().unwrap().to_owned();
    let mut from_map = from_val.as_object().unwrap().to_owned();
    base_map.append(&mut from_map);

    Ok(serde_json::from_value::<T>(serde_json::to_value(
        base_map,
    )?)?)
}
