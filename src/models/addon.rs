use crate::models::enums::ProductKind;

/// A lightweight representation of a Store add-on (DLC / IAP).
#[derive(Debug, Clone)]
pub struct Addon {
    pub product_id: String,
    pub product_type: ProductKind,
    pub display_name: String,
}
