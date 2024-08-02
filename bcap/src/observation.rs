struct Observation {
    position: Position,
    transmitter: Transmitter,
}

#[non_exhaustive]
enum Transmitter {
    WiFi(WiFiTransmitter),
    // todo: cell, bluetooth
}

struct WiFiTransmitter {
    read_key: u32,
    write_key: u32,
    signal: i32,
}

struct Position {
    latitude: f64,
    longitude: f64,
    accuracy: Option<f64>,
    altitude: Option<f64>,
    altitude_accuracy: Option<f64>,
    speed: Option<f64>,
}
