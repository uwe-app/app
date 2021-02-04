use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::rc::Rc;
use xml::{
    name::OwnedName,
    reader::{EventReader, XmlEvent},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::{Error, Result};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(untagged)]
pub enum XmlChildren {
    One(Rc<RefCell<XmlValue>>),
    Many(Vec<Rc<RefCell<XmlValue>>>),
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct XmlValue {
    #[serde(skip)]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cdata: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub attributes: Map<String, Value>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub children: HashMap<String, XmlChildren>,
}

impl XmlValue {
    pub fn new(name: String) -> Self {
        Self {
            name,
            attributes: Map::new(),
            children: HashMap::new(),
            cdata: None,
            text: None,
        }
    }

    pub fn new_attributes(
        name: String,
        attributes: Map<String, Value>,
    ) -> Self {
        Self {
            name,
            attributes,
            children: HashMap::new(),
            cdata: None,
            text: None,
        }
    }
}

fn qualified_xml_name(name: OwnedName) -> String {
    if let Some(prefix) = name.prefix {
        format!("{}:{}", prefix, name.local_name)
    } else {
        name.local_name
    }
}

pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Value> {
    let doc = File::open(path.as_ref())?;
    if let Some(doc) = from_read(doc)? {
        Ok(json!(doc))
    } else {
        Err(Error::EmptyXmlDocument(path.as_ref().to_path_buf()))
    }
}

fn from_read<R>(inner: R) -> Result<Option<Rc<RefCell<XmlValue>>>>
where
    R: Read,
{
    convert(inner)
}

fn convert<R>(inner: R) -> Result<Option<Rc<RefCell<XmlValue>>>>
where
    R: Read,
{
    let reader = EventReader::new(inner);
    let document = XmlValue::new("document".to_string());

    let mut parents = vec![Rc::new(RefCell::new(document))];
    let mut response: Option<Rc<RefCell<XmlValue>>> = None;
    let mut current: Option<Rc<RefCell<XmlValue>>> = None;

    for result in reader {
        let event = result?;
        match event {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                if let Some(ref mut parent) = parents.last_mut() {
                    let node_name = qualified_xml_name(name);

                    let value = if attributes.is_empty() {
                        Rc::new(RefCell::new(XmlValue::new(node_name.clone())))
                    } else {
                        let attrs = attributes
                            .into_iter()
                            .map(|attr| {
                                (
                                    qualified_xml_name(attr.name),
                                    Value::String(attr.value),
                                )
                            })
                            .collect::<Map<String, Value>>();

                        Rc::new(RefCell::new(XmlValue::new_attributes(
                            node_name.clone(),
                            attrs,
                        )))
                    };

                    current = Some(Rc::clone(&value));

                    let insert = Rc::clone(&value);
                    let mut writer = parent.borrow_mut();
                    if let Some(child_list) =
                        writer.children.get_mut(&node_name)
                    {
                        match child_list {
                            XmlChildren::One(node) => {
                                let mut nodes = vec![node.clone()];
                                nodes.push(insert);
                                writer.children.insert(
                                    node_name,
                                    XmlChildren::Many(nodes),
                                );
                            }
                            XmlChildren::Many(nodes) => {
                                nodes.push(insert);
                            }
                        }
                    } else {
                        writer
                            .children
                            .insert(node_name, XmlChildren::One(insert));
                    }
                    drop(writer);
                    parents.push(value);
                }
            }
            XmlEvent::CData(value) => {
                if let Some(ref mut current) = current {
                    let mut writer = current.borrow_mut();
                    writer.cdata = Some(value);
                }
            }

            XmlEvent::Characters(value) => {
                if let Some(ref mut current) = current {
                    let mut writer = current.borrow_mut();
                    writer.text = Some(value);
                }
            }
            XmlEvent::EndElement { .. } => {
                response = parents.pop();
                current = None;
            }
            XmlEvent::EndDocument => {
                break;
            }
            _ => {}
        }
    }

    Ok(response)
}

/*
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct XmlValue {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cdata: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub attributes: Map<String, Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Rc<RefCell<XmlValue>>>,
}

impl XmlValue {
    pub fn new(name: String) -> Self {
        Self {
            name,
            attributes: Map::new(),
            children: Vec::new(),
            cdata: None,
            text: None,
        }
    }

    pub fn new_attributes(
        name: String,
        attributes: Map<String, Value>,
    ) -> Self {
        Self {
            name,
            attributes,
            children: Vec::new(),
            cdata: None,
            text: None,
        }
    }
}

fn qualified_xml_name(name: OwnedName) -> String {
    if let Some(prefix) = name.prefix {
        format!("{}:{}", prefix, name.local_name)
    } else {
        name.local_name
    }
}

pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Value> {
    let doc = File::open(path.as_ref())?;
    if let Some(doc) = from_read(doc)? {
        Ok(json!(doc))
    } else {
        Err(Error::EmptyXmlDocument(path.as_ref().to_path_buf()))
    }
}

fn from_read<R>(inner: R) -> Result<Option<Rc<RefCell<XmlValue>>>>
where
    R: Read,
{
    convert(inner)
}

fn convert<R>(inner: R) -> Result<Option<Rc<RefCell<XmlValue>>>>
where
    R: Read,
{
    let reader = EventReader::new(inner);
    let document = XmlValue::new("document".to_string());

    let mut parents = vec![Rc::new(RefCell::new(document))];
    let mut response: Option<Rc<RefCell<XmlValue>>> = None;
    let mut current: Option<Rc<RefCell<XmlValue>>> = None;

    for result in reader {
        let event = result?;
        match event {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                if let Some(ref mut parent) = parents.last_mut() {
                    let value = if attributes.is_empty() {
                        Rc::new(RefCell::new(XmlValue::new(
                            qualified_xml_name(name),
                        )))
                    } else {
                        let attrs = attributes
                            .into_iter()
                            .map(|attr| {
                                (
                                    qualified_xml_name(attr.name),
                                    Value::String(attr.value),
                                )
                            })
                            .collect::<Map<String, Value>>();

                        Rc::new(RefCell::new(XmlValue::new_attributes(
                            qualified_xml_name(name),
                            attrs,
                        )))
                    };

                    current = Some(Rc::clone(&value));

                    let insert = Rc::clone(&value);
                    let mut writer = parent.borrow_mut();
                    writer.children.push(insert);
                    drop(writer);
                    parents.push(value);
                }
            }
            XmlEvent::EndElement { .. } => {
                response = parents.pop();
            }
            XmlEvent::CData(value) => {
                if let Some(ref mut current) = current {
                    let mut writer = current.borrow_mut();
                    writer.cdata = Some(value);
                }
            }
            XmlEvent::Characters(value) => {
                if let Some(ref mut current) = current {
                    let mut writer = current.borrow_mut();
                    writer.text = Some(value);
                }
            }
            XmlEvent::EndDocument => {
                break;
            }
            _ => {}
        }
    }

    Ok(response)
}
*/
