use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Deserializer, Serialize, de};
use seruq_ucr::{
    AssetPath, Element, ElementId, ElementKind, FfrCall, FontSize, Metadata, Point, Size, Timing,
    UcrDraft,
};

use crate::{
    Error,
    scalar::{JsonColor, JsonFloat, JsonId},
    value::JsonValueMap,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UcrJson {
    metadata: MetadataJson,
    root: JsonId,
    #[serde(deserialize_with = "deserialize_elements")]
    elements: BTreeMap<JsonId, ElementJson>,
    #[serde(default, skip_serializing_if = "JsonValueMap::is_empty")]
    state: JsonValueMap,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MetadataJson {
    version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_font: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    background: Option<JsonColor>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ElementJson {
    position: PointJson,
    size: SizeJson,
    kind: ElementKindJson,
    #[serde(
        default,
        deserialize_with = "deserialize_actions",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    actions: BTreeMap<TimingJson, FfrCallJson>,
    #[serde(default, skip_serializing_if = "JsonValueMap::is_empty")]
    data: JsonValueMap,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum TimingJson {
    Pushed,
    Released,
    Activated,
    Focused,
    Unfocused,
    ValueChanged,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FfrCallJson {
    function_name: String,
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        background: Option<JsonColor>,
    },
    Text {
        text: String,
        font_size: JsonFloat,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        font: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<JsonColor>,
    },
    Image {
        source: String,
    },
    Button {
        label: String,
        font_size: JsonFloat,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        font: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<JsonColor>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        background: Option<JsonColor>,
    },
    TextInput {
        value: String,
        font_size: JsonFloat,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        font: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<JsonColor>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        background: Option<JsonColor>,
    },
    Geometry {},
}

impl From<&UcrDraft> for UcrJson {
    fn from(draft: &UcrDraft) -> Self {
        Self {
            metadata: MetadataJson {
                version: draft.metadata.version().to_owned(),
                default_font: draft.metadata.default_font().map(asset_to_json),
                background: draft.metadata.background().map(JsonColor),
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
                            actions: element
                                .actions
                                .iter()
                                .map(|(&timing, call)| {
                                    (
                                        timing.into(),
                                        FfrCallJson {
                                            function_name: call.function_name.clone(),
                                        },
                                    )
                                })
                                .collect(),
                            data: JsonValueMap(element.data.clone()),
                        },
                    )
                })
                .collect(),
            state: JsonValueMap(draft.state.clone()),
        }
    }
}

impl TryFrom<UcrJson> for UcrDraft {
    type Error = Error;

    fn try_from(document: UcrJson) -> Result<Self, Self::Error> {
        let elements = document
            .elements
            .into_iter()
            .map(|(id, element)| {
                let position =
                    Point::new(element.position.x.0, element.position.y.0).map_err(|reason| {
                        Error::InvalidGeometry {
                            element: id.0,
                            field: "position",
                            reason,
                        }
                    })?;
                let size =
                    Size::new(element.size.width.0, element.size.height.0).map_err(|reason| {
                        Error::InvalidGeometry {
                            element: id.0,
                            field: "size",
                            reason,
                        }
                    })?;
                Ok((
                    id.0,
                    Element {
                        position,
                        size,
                        kind: element.kind.into_kind(id.0)?,
                        actions: element
                            .actions
                            .into_iter()
                            .map(|(timing, call)| (timing.into(), FfrCall::new(call.function_name)))
                            .collect(),
                        data: element.data.0,
                    },
                ))
            })
            .collect::<Result<_, Error>>()?;

        let default_font = document
            .metadata
            .default_font
            .map(|path| asset_from_json(path, None, "default_font"))
            .transpose()?;

        let mut metadata = Metadata::new(document.metadata.version, default_font);
        if let Some(background) = document.metadata.background {
            metadata = metadata.with_background(background.0);
        }
        Ok(Self {
            metadata,
            root: document.root.0,
            elements,
            state: document.state.0,
        })
    }
}

impl From<&ElementKind> for ElementKindJson {
    fn from(kind: &ElementKind) -> Self {
        match kind {
            ElementKind::Container {
                children,
                background,
            } => Self::Container {
                children: children.iter().copied().map(JsonId).collect(),
                background: background.map(JsonColor),
            },
            ElementKind::Text {
                text,
                font_size,
                font,
                color,
            } => Self::Text {
                text: text.clone(),
                font_size: JsonFloat(font_size.get()),
                font: font.as_ref().map(asset_to_json),
                color: color.map(JsonColor),
            },
            ElementKind::Image { source } => Self::Image {
                source: asset_to_json(source),
            },
            ElementKind::Button {
                label,
                font_size,
                font,
                color,
                background,
            } => Self::Button {
                label: label.clone(),
                font_size: JsonFloat(font_size.get()),
                font: font.as_ref().map(asset_to_json),
                color: color.map(JsonColor),
                background: background.map(JsonColor),
            },
            ElementKind::TextInput {
                value,
                font_size,
                font,
                placeholder,
                color,
                background,
            } => Self::TextInput {
                value: value.clone(),
                font_size: JsonFloat(font_size.get()),
                font: font.as_ref().map(asset_to_json),
                placeholder: placeholder.clone(),
                color: color.map(JsonColor),
                background: background.map(JsonColor),
            },
            ElementKind::Geometry => Self::Geometry {},
        }
    }
}

impl ElementKindJson {
    fn into_kind(self, id: ElementId) -> Result<ElementKind, Error> {
        let font_size = |value: JsonFloat| {
            FontSize::new(value.0).map_err(|reason| Error::InvalidFontSize {
                element: id,
                reason,
            })
        };
        let font = |path: Option<String>| {
            path.map(|path| asset_from_json(path, Some(id), "font"))
                .transpose()
        };

        Ok(match self {
            Self::Container {
                children,
                background,
            } => ElementKind::Container {
                children: children.into_iter().map(|id| id.0).collect(),
                background: background.map(|color| color.0),
            },
            Self::Text {
                text,
                font_size: size,
                font: path,
                color,
            } => ElementKind::Text {
                text,
                font_size: font_size(size)?,
                font: font(path)?,
                color: color.map(|color| color.0),
            },
            Self::Image { source } => ElementKind::Image {
                source: asset_from_json(source, Some(id), "source")?,
            },
            Self::Button {
                label,
                font_size: size,
                font: path,
                color,
                background,
            } => ElementKind::Button {
                label,
                font_size: font_size(size)?,
                font: font(path)?,
                color: color.map(|color| color.0),
                background: background.map(|color| color.0),
            },
            Self::TextInput {
                value,
                font_size: size,
                font: path,
                placeholder,
                color,
                background,
            } => ElementKind::TextInput {
                value,
                font_size: font_size(size)?,
                font: font(path)?,
                placeholder,
                color: color.map(|color| color.0),
                background: background.map(|color| color.0),
            },
            Self::Geometry {} => ElementKind::Geometry,
        })
    }
}

fn asset_to_json(path: &AssetPath) -> String {
    path.as_str().to_owned()
}

fn asset_from_json(
    path: String,
    element: Option<ElementId>,
    field: &'static str,
) -> Result<AssetPath, Error> {
    AssetPath::new(path).map_err(|reason| Error::InvalidAssetPath {
        element,
        field,
        reason,
    })
}

impl From<Timing> for TimingJson {
    fn from(timing: Timing) -> Self {
        match timing {
            Timing::Pushed => Self::Pushed,
            Timing::Released => Self::Released,
            Timing::Activated => Self::Activated,
            Timing::Focused => Self::Focused,
            Timing::Unfocused => Self::Unfocused,
            Timing::ValueChanged => Self::ValueChanged,
        }
    }
}

impl From<TimingJson> for Timing {
    fn from(timing: TimingJson) -> Self {
        match timing {
            TimingJson::Pushed => Self::Pushed,
            TimingJson::Released => Self::Released,
            TimingJson::Activated => Self::Activated,
            TimingJson::Focused => Self::Focused,
            TimingJson::Unfocused => Self::Unfocused,
            TimingJson::ValueChanged => Self::ValueChanged,
        }
    }
}

fn deserialize_actions<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<TimingJson, FfrCallJson>, D::Error> {
    struct ActionsVisitor;

    impl<'de> de::Visitor<'de> for ActionsVisitor {
        type Value = BTreeMap<TimingJson, FfrCallJson>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an object mapping unique timings to FFR calls")
        }

        fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut actions = BTreeMap::new();
            while let Some((timing, call)) = map.next_entry::<TimingJson, FfrCallJson>()? {
                if actions.insert(timing, call).is_some() {
                    return Err(de::Error::custom("duplicate action timing"));
                }
            }
            Ok(actions)
        }
    }

    deserializer.deserialize_map(ActionsVisitor)
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
