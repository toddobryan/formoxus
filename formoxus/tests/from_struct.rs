pub struct Event {
    pub id: u32,              // server-assigned — not collected by any form field
    pub title: String,
    pub location: Location,
}

pub struct Location {
    pub street: String,
    pub city: String,
    pub zip: String,
}

EventForm = struct_form!{
    Event {
        id: None,
        title: String,
        location: FieldSet<Location> {
            street: String,
            city: String,
            zip: String,
        }
    }
}

pub struct EventForm {
    pub id: u32,
    pub title: String,
    pub location: LocationFieldSet,
}

pub struct LocationFieldSet {
    pub street: String,
    pub city: String,
    pub zip: String,
}