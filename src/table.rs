use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TableId {
    Flights,
    Missions,
    Drones,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableColumnType {
    Text,
    Textarea,
    Number,
    Date,
    Select,
}

#[derive(Debug, Clone, Copy)]
pub struct TableColumnSchema {
    pub name: &'static str,
    pub display_name: &'static str,
    pub column_type: TableColumnType,
    pub required: bool,
    pub reference: Option<TableId>,
    pub options: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub struct TableSchema {
    pub id: TableId,
    pub display_name: &'static str,
    pub columns: &'static [TableColumnSchema],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRow {
    pub cells: HashMap<String, String>,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct TableFileManagerTarget {
    pub table: TableId,
    pub row_index: usize,
}

const STATE_OPTIONS: &[&str] = &["Готов к вылету", "Уничтожен"];

const FLIGHTS_COLUMNS: &[TableColumnSchema] = &[
    TableColumnSchema {
        name: "mission",
        display_name: "Миссия",
        column_type: TableColumnType::Select,
        required: true,
        reference: Some(TableId::Missions),
        options: &[],
    },
    TableColumnSchema {
        name: "drone",
        display_name: "Борт",
        column_type: TableColumnType::Select,
        required: true,
        reference: Some(TableId::Drones),
        options: &[],
    },
    TableColumnSchema {
        name: "state",
        display_name: "Состояние",
        column_type: TableColumnType::Select,
        required: false,
        reference: None,
        options: STATE_OPTIONS,
    },
    TableColumnSchema {
        name: "catapultNumber",
        display_name: "Номер катапульты",
        column_type: TableColumnType::Text,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "dateShipment",
        display_name: "Дата отгрузки",
        column_type: TableColumnType::Date,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "whoAccepted",
        display_name: "Кто принял",
        column_type: TableColumnType::Text,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "testDate",
        display_name: "Дата испытаний",
        column_type: TableColumnType::Date,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "bvsLayout",
        display_name: "Компоновка БВС",
        column_type: TableColumnType::Text,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "curbWeight",
        display_name: "Снаряженная масса (кг)",
        column_type: TableColumnType::Number,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "testTeam",
        display_name: "Группа испытателей",
        column_type: TableColumnType::Textarea,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "polygon",
        display_name: "Полигон",
        column_type: TableColumnType::Text,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "versionSVP",
        display_name: "Версия СВП",
        column_type: TableColumnType::Text,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "versionFPO",
        display_name: "Версия ФПО",
        column_type: TableColumnType::Text,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "petrol",
        display_name: "Бензин АИ-92 (л)",
        column_type: TableColumnType::Number,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "oil",
        display_name: "Масло 2Т (л)",
        column_type: TableColumnType::Number,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "result",
        display_name: "Результат полета",
        column_type: TableColumnType::Number,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "flightTime",
        display_name: "Время полета (мин)",
        column_type: TableColumnType::Number,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "takeoffSpeed",
        display_name: "Обороты на взлетном",
        column_type: TableColumnType::Number,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "windTemperature",
        display_name: "Температура ветра (°C)",
        column_type: TableColumnType::Number,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "humidity",
        display_name: "Влажность (%)",
        column_type: TableColumnType::Number,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "precipitation",
        display_name: "Осадки",
        column_type: TableColumnType::Text,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "availabilityReport",
        display_name: "Наличие отчета",
        column_type: TableColumnType::Text,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "transferCertificate",
        display_name: "Акт передачи",
        column_type: TableColumnType::Text,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "comments",
        display_name: "Комментарии",
        column_type: TableColumnType::Textarea,
        required: false,
        reference: None,
        options: &[],
    },
];

const MISSIONS_COLUMNS: &[TableColumnSchema] = &[
    TableColumnSchema {
        name: "missionObjective",
        display_name: "Цель миссии",
        column_type: TableColumnType::Textarea,
        required: true,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "drone",
        display_name: "Борт",
        column_type: TableColumnType::Select,
        required: true,
        reference: Some(TableId::Drones),
        options: &[],
    },
];

const DRONES_COLUMNS: &[TableColumnSchema] = &[
    TableColumnSchema {
        name: "factoryNumber",
        display_name: "Заводской номер",
        column_type: TableColumnType::Text,
        required: true,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "alternativeNumber",
        display_name: "Альтернативный номер",
        column_type: TableColumnType::Text,
        required: false,
        reference: None,
        options: &[],
    },
    TableColumnSchema {
        name: "state",
        display_name: "Состояние",
        column_type: TableColumnType::Select,
        required: false,
        reference: None,
        options: STATE_OPTIONS,
    },
];

const FLIGHTS_SCHEMA: TableSchema = TableSchema {
    id: TableId::Flights,
    display_name: "Полеты",
    columns: FLIGHTS_COLUMNS,
};

const MISSIONS_SCHEMA: TableSchema = TableSchema {
    id: TableId::Missions,
    display_name: "Миссии",
    columns: MISSIONS_COLUMNS,
};

const DRONES_SCHEMA: TableSchema = TableSchema {
    id: TableId::Drones,
    display_name: "Борты",
    columns: DRONES_COLUMNS,
};

pub fn table_schema(table: TableId) -> &'static TableSchema {
    match table {
        TableId::Flights => &FLIGHTS_SCHEMA,
        TableId::Missions => &MISSIONS_SCHEMA,
        TableId::Drones => &DRONES_SCHEMA,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_table_has_non_empty_schema() {
        for table in [TableId::Flights, TableId::Missions, TableId::Drones] {
            let schema = table_schema(table);
            assert_eq!(schema.id, table);
            assert!(!schema.display_name.trim().is_empty());
            assert!(!schema.columns.is_empty());
        }
    }

    #[test]
    fn flight_schema_contains_required_links() {
        let schema = table_schema(TableId::Flights);
        let mission = schema.columns.iter().find(|c| c.name == "mission");
        let drone = schema.columns.iter().find(|c| c.name == "drone");

        assert!(mission.is_some());
        assert!(drone.is_some());
        assert_eq!(mission.unwrap().reference, Some(TableId::Missions));
        assert_eq!(drone.unwrap().reference, Some(TableId::Drones));
    }

    #[test]
    fn drones_state_column_uses_select_options() {
        let schema = table_schema(TableId::Drones);
        let state = schema
            .columns
            .iter()
            .find(|c| c.name == "state")
            .expect("state column");

        assert_eq!(state.column_type, TableColumnType::Select);
        assert!(state.options.contains(&"Готов к вылету"));
        assert!(state.options.contains(&"Уничтожен"));
    }
}
