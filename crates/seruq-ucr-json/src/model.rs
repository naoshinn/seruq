use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Deserializer, Serialize, de};
use seruq_ucr::{Element, ElementKind, Metadata, Point, Size, UcrDraft};

use crate::{
    Error,
    scalar::{JsonFloat, JsonId},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UcrJson {
    metadata: MetadataJson,
    root: JsonId,
    #[serde(deserialize_with = "deserialize_elements")]
    elements: BTreeMap<JsonId, ElementJson>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MetadataJson {
    version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_font: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ElementJson {
    position: PointJson,
    size: SizeJson,
    kind: ElementKindJson,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PointJson {
    x: JsonFloat,
    y: JsonFloat,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SizeJson {
    width: JsonFloat,
    height: JsonFloat,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum ElementKindJson {
    Container {
        children: Vec<JsonId>,
    },
    Text {
        text: String,
        font_size: JsonFloat,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        font: Option<String>,
    },
    Image {
        source: String,
    },
    Button {
        label: String,
        font_size: JsonFloat,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        font: Option<String>,
    },
    TextInput {
        value: String,
        font_size: JsonFloat,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        font: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
    },
    Geometry {},
}

impl From<&UcrDraft> for UcrJson {
    fn from(draft: &UcrDraft) -> Self {
        Self {
            metadata: MetadataJson {
                version: draft.metadata.version().to_owned(),
                default_font: draft.metadata.default_font().map(str::to_owned),
            },
            root: JsonId(draft.root),
            elements: draft
                .elements
                .iter()
                .map(|(&id, element)| {
                    (
                        JsonId(id),
                        ElementJson {
                            position: PointJson {
                                x: JsonFloat(element.position.x()),
                                y: JsonFloat(element.position.y()),
                            },
                            size: SizeJson {
                                width: JsonFloat(element.size.width()),
                                height: JsonFloat(element.size.height()),
                            },
                            kind: (&element.kind).into(),
                        },
                    )
                })
                .collect(),
        }
    }
}

impl TryFrom<UcrJson> for UcrDraft {
    type Error = Error;

    fn try_from(document: UcrJson) -> Result<Self, Self::Error> {
        let elements =
            document
                .elements
                .into_iter()
                .map(|(id, element)| {
                    let position = Point::new(element.position.x.0, element.position.y.0).map_err(
                        |reason| Error::InvalidGeometry {
                            element: id.0,
                            field: "position",
                            reason,
                        },
                    )?;
                    let size = Size::new(element.size.width.0, element.size.height.0).map_err(
                        |reason| Error::InvalidGeometry {
                            element: id.0,
                            field: "size",
                            reason,
                        },
                    )?;
                    Ok((
                        id.0,
                        Element {
                            position,
                            size,
                            kind: element.kind.into(),
                        },
                    ))
                })
                .collect::<Result<_, Error>>()?;

        Ok(Self {
            metadata: Metadata::new(document.metadata.version, document.metadata.default_font),
            root: document.root.0,
            elements,
        })
    }
}

impl From<&ElementKind> for ElementKindJson {
    fn from(kind: &ElementKind) -> Self {
        match kind {
            ElementKind::Container { children } => Self::Container {
                children: children.iter().copied().map(JsonId).collect(),
            },
            ElementKind::Text {
                text,
                font_size,
                font,
            } => Self::Text {
                text: text.clone(),
                font_size: JsonFloat(*font_size),
                font: font.clone(),
            },
            ElementKind::Image { source } => Self::Image {
                source: source.clone(),
            },
            ElementKind::Button {
                label,
                font_size,
                font,
            } => Self::Button {
                label: label.clone(),
                font_size: JsonFloat(*font_size),
                font: font.clone(),
            },
            ElementKind::TextInput {
                value,
                font_size,
                font,
                placeholder,
            } => Self::TextInput {
                value: value.clone(),
                font_size: JsonFloat(*font_size),
                font: font.clone(),
                placeholder: placeholder.clone(),
            },
            ElementKind::Geometry => Self::Geometry {},
        }
    }
}

impl From<ElementKindJson> for ElementKind {
    fn from(kind: ElementKindJson) -> Self {
        match kind {
            ElementKindJson::Container { children } => Self::Container {
                children: children.into_iter().map(|id| id.0).collect(),
            },
            ElementKindJson::Text {
                text,
                font_size,
                font,
            } => Self::Text {
                text,
                font_size: font_size.0,
                font,
            },
            ElementKindJson::Image { source } => Self::Image { source },
            ElementKindJson::Button {
                label,
                font_size,
                font,
            } => Self::Button {
                label,
                font_size: font_size.0,
                font,
            },
            ElementKindJson::TextInput {
                value,
                font_size,
                font,
                placeholder,
            } => Self::TextInput {
                value,
                font_size: font_size.0,
                font,
                placeholder,
            },
            ElementKindJson::Geometry {} => Self::Geometry,
        }
    }
}

// A normal map deserializer silently overwrites duplicate IDs.
fn deserialize_elements<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<JsonId, ElementJson>, D::Error> {
    struct ElementsVisitor;

    impl<'de> de::Visitor<'de> for ElementsVisitor {
        type Value = BTreeMap<JsonId, ElementJson>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an object mapping unique decimal IDs to elements")
        }

        fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut elements = BTreeMap::new();
            while let Some((id, element)) = map.next_entry::<JsonId, ElementJson>()? {
                if elements.insert(id, element).is_some() {
                    return Err(de::Error::custom(format!("duplicate element ID {}", id.0)));
                }
            }
            Ok(elements)
        }
    }

    deserializer.deserialize_map(ElementsVisitor)
}
