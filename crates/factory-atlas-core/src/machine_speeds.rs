use crate::model::{MachineDef, MachineSpeedMode, RecipeDef, SpeedFormulaId, SpeedSource};

pub const TICKS_PER_MINUTE: f64 = 1200.0;

#[derive(Debug, Clone)]
pub struct VerifiedSpeedFormulaSpec {
    pub id: SpeedFormulaId,
    pub name_ru: &'static str,
    pub name_en: &'static str,
    pub supports_rpm: bool,
    pub default_stress_su: Option<f64>,
    pub note_ru: &'static str,
    pub note_en: &'static str,
}

pub fn verified_speed_formulas() -> &'static [VerifiedSpeedFormulaSpec] {
    &[
        VerifiedSpeedFormulaSpec {
            id: SpeedFormulaId::Unknown,
            name_ru: "Неизвестно / задать вручную (честно)",
            name_en: "Unknown / Manual rate (honest)",
            supports_rpm: false,
            default_stress_su: None,
            note_ru: "Скорость задаётся вручную пользователем или остаётся неизвестной.",
            note_en: "Speed is user-entered manually or left explicitly unknown.",
        },
        VerifiedSpeedFormulaSpec {
            id: SpeedFormulaId::Millstone,
            name_ru: "Create 6.0.11: Жёрнов (MillstoneBlockEntity)",
            name_en: "Create 6.0.11: Millstone",
            supports_rpm: true,
            default_stress_su: Some(4.0),
            note_ru: "timer -= clamp(floor(|RPM|/16), 1, 512)",
            note_en: "timer -= clamp(floor(|RPM|/16), 1, 512)",
        },
        VerifiedSpeedFormulaSpec {
            id: SpeedFormulaId::MechanicalPress,
            name_ru: "Create 6.0.11: Механический пресс (PressingBehaviour)",
            name_en: "Create 6.0.11: Mechanical Press",
            supports_rpm: true,
            default_stress_su: Some(8.0),
            note_ru: "CYCLE=240, шаг=lerp(|RPM|/512, 1, 60)",
            note_en: "CYCLE=240, tickSpeed=lerp(|RPM|/512, 1, 60)",
        },
        VerifiedSpeedFormulaSpec {
            id: SpeedFormulaId::MechanicalMixer,
            name_ru: "Create 6.0.11: Механический смешиватель (Mixer)",
            name_en: "Create 6.0.11: Mechanical Mixer",
            supports_rpm: true,
            default_stress_su: Some(4.0),
            note_ru: "40 + max(log2(512/|RPM|)*15*(dur/100)+1, 1)",
            note_en: "40 + max(log2(512/|RPM|)*15*(dur/100)+1, 1)",
        },
        VerifiedSpeedFormulaSpec {
            id: SpeedFormulaId::MechanicalSaw,
            name_ru: "Create 6.0.11: Механическая пила (Saw)",
            name_en: "Create 6.0.11: Mechanical Saw",
            supports_rpm: true,
            default_stress_su: Some(4.0),
            note_ru: "processingSpeed = clamp(|RPM|/24, 1, 128)",
            note_en: "processingSpeed = clamp(|RPM|/24, 1, 128)",
        },
        VerifiedSpeedFormulaSpec {
            id: SpeedFormulaId::Deployer,
            name_ru: "Create 6.0.11: Автономный активатор (Deployer)",
            name_en: "Create 6.0.11: Deployer",
            supports_rpm: true,
            default_stress_su: Some(4.0),
            note_ru: "timerSpeed = clamp(|RPM|*2, 8, 512), цикл 2500",
            note_en: "timerSpeed = clamp(|RPM|*2, 8, 512), 2500 ticks",
        },
        VerifiedSpeedFormulaSpec {
            id: SpeedFormulaId::Spout,
            name_ru: "Create 6.0.11: Дозатор (Spout, 20 тиков)",
            name_en: "Create 6.0.11: Spout (20 ticks)",
            supports_rpm: false,
            default_stress_su: Some(0.0),
            note_ru: "FILLING_TIME = 20 тиков (без кинетики)",
            note_en: "FILLING_TIME = 20 ticks (no kinetic load)",
        },
        VerifiedSpeedFormulaSpec {
            id: SpeedFormulaId::ItemDrain,
            name_ru: "Create 6.0.11: Осушитель предметов (Item Drain, 20 тиков)",
            name_en: "Create 6.0.11: Item Drain (20 ticks)",
            supports_rpm: false,
            default_stress_su: Some(0.0),
            note_ru: "FILLING_TIME = 20 тиков",
            note_en: "FILLING_TIME = 20 ticks",
        },
        VerifiedSpeedFormulaSpec {
            id: SpeedFormulaId::CrushingWheel,
            name_ru: "Create 6.0.11: Дробильные колёса (допущение пачки)",
            name_en: "Create 6.0.11: Crushing Wheels (batch assumption)",
            supports_rpm: true,
            default_stress_su: Some(16.0),
            note_ru: "Зависит от RPM и размера пачки (помечается как допущение)",
            note_en: "Depends on RPM & batch size (marked assumption)",
        },
        VerifiedSpeedFormulaSpec {
            id: SpeedFormulaId::EncasedFan,
            name_ru: "Create 6.0.11: Вентилятор в корпусе (допущение пачки)",
            name_en: "Create 6.0.11: Encased Fan (batch assumption)",
            supports_rpm: true,
            default_stress_su: Some(2.0),
            note_ru: "150 тиков * ((пачка-1)/16 + 1) (помечается как допущение)",
            note_en: "150 ticks * ((stack-1)/16 + 1) (marked assumption)",
        },
        VerifiedSpeedFormulaSpec {
            id: SpeedFormulaId::MechanicalCrafter,
            name_ru: "Create 6.0.11: Механический сборщик (допущение)",
            name_en: "Create 6.0.11: Mechanical Crafter (assumption)",
            supports_rpm: true,
            default_stress_su: Some(2.0),
            note_ru: "Зависит от числа ячеек сетки и RPM (допущение)",
            note_en: "Depends on grid cell count and RPM (assumption)",
        },
        VerifiedSpeedFormulaSpec {
            id: SpeedFormulaId::FixedTicks,
            name_ru: "Фиксированная длительность из рецепта (тики)",
            name_en: "Fixed ticks from recipe duration",
            supports_rpm: false,
            default_stress_su: Some(0.0),
            note_ru: "Использует processingTimeTicks рецепта (1200 тиков = 1 мин)",
            note_en: "Uses recipe processingTimeTicks (1200 ticks = 1 min)",
        },
    ]
}

#[derive(Debug, Clone)]
pub struct MachineSpeedCalculation {
    pub crafts_per_min: Option<f64>,
    pub ticks_per_craft: Option<f64>,
    pub source: SpeedSource,
    pub stress_per_machine_su: Option<f64>,
    pub explanation_ru: String,
    pub explanation_en: String,
}

fn clamp(val: f64, min: f64, max: f64) -> f64 {
    val.max(min).min(max)
}

fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + (b - a) * clamp(t, 0.0, 1.0)
}

pub fn compute_machine_throughput(
    machine: Option<&MachineDef>,
    recipe: Option<&RecipeDef>,
    rpm: Option<f64>,
    speed_mode: Option<MachineSpeedMode>,
    manual_crafts_per_min: Option<f64>,
    stack_size_assumption: Option<f64>,
) -> MachineSpeedCalculation {
    let mode = speed_mode.unwrap_or(MachineSpeedMode::Formula);

    if mode == MachineSpeedMode::Manual {
        if let Some(mrate) = manual_crafts_per_min
            && mrate.is_finite()
            && mrate > 0.0
        {
            let rpm_eff = rpm.or_else(|| machine.and_then(|m| m.default_rpm)).unwrap_or(0.0).abs();
            let su = machine
                .and_then(|m| m.stress_impact_su)
                .map(|s| s * rpm_eff);
            return MachineSpeedCalculation {
                crafts_per_min: Some(mrate),
                ticks_per_craft: Some(TICKS_PER_MINUTE / mrate),
                source: SpeedSource::Manual,
                stress_per_machine_su: su,
                explanation_ru: format!("Ручная скорость пользователя: {mrate:.2} опер./мин."),
                explanation_en: format!("User manual rate: {mrate:.2} ops/min."),
            };
        }
        return MachineSpeedCalculation {
            crafts_per_min: None,
            ticks_per_craft: None,
            source: SpeedSource::Unknown,
            stress_per_machine_su: None,
            explanation_ru: "Выбран ручной режим скорости, но значение (опер./мин) не задано."
                .to_owned(),
            explanation_en: "Manual speed mode selected, but ops/min is missing.".to_owned(),
        };
    }

    let Some(machine) = machine else {
        return MachineSpeedCalculation {
            crafts_per_min: None,
            ticks_per_craft: None,
            source: SpeedSource::Unknown,
            stress_per_machine_su: None,
            explanation_ru: "Машина не назначена — скорость неизвестна.".to_owned(),
            explanation_en: "No machine assigned — speed unknown.".to_owned(),
        };
    };

    if mode == MachineSpeedMode::Unknown || machine.speed_formula_id == SpeedFormulaId::Unknown {
        return MachineSpeedCalculation {
            crafts_per_min: None,
            ticks_per_craft: None,
            source: SpeedSource::Unknown,
            stress_per_machine_su: None,
            explanation_ru: format!(
                "Точная скорость для «{}» не подтверждена. Укажите ручной темп.",
                machine.name_ru
            ),
            explanation_en: format!(
                "Exact processing timing for \"{}\" is unknown. Enter manual ops/min.",
                machine.name_en
            ),
        };
    }

    let rpm_raw = rpm.or(machine.default_rpm).unwrap_or(64.0).abs();
    let effective_rpm = if machine.supports_rpm {
        clamp(rpm_raw, 0.0, machine.max_rpm)
    } else {
        0.0
    };

    if machine.supports_rpm && effective_rpm <= 0.0 {
        return MachineSpeedCalculation {
            crafts_per_min: Some(0.0),
            ticks_per_craft: None,
            source: SpeedSource::ExactFormula,
            stress_per_machine_su: Some(0.0),
            explanation_ru: "0 RPM: механизм остановлен.".to_owned(),
            explanation_en: "0 RPM: machine is halted.".to_owned(),
        };
    }

    let su_per_machine = machine.stress_impact_su.map(|su| {
        if machine.supports_rpm {
            su * effective_rpm
        } else {
            su
        }
    });

    let base_dur = recipe
        .and_then(|r| r.processing_time_ticks)
        .filter(|d| *d > 0.0)
        .unwrap_or(100.0);

    match machine.speed_formula_id {
        SpeedFormulaId::Millstone => {
            let step = clamp((effective_rpm / 16.0).floor(), 1.0, 512.0);
            let ticks = (base_dur / step).ceil().max(1.0);
            MachineSpeedCalculation {
                crafts_per_min: Some(TICKS_PER_MINUTE / ticks),
                ticks_per_craft: Some(ticks),
                source: SpeedSource::ExactFormula,
                stress_per_machine_su: su_per_machine,
                explanation_ru: format!(
                    "Create 6.0.11 Millstone: шаг {step} за тик ({effective_rpm} RPM), {ticks} тиков/опер."
                ),
                explanation_en: format!(
                    "Create 6.0.11 Millstone: step {step}/tick at {effective_rpm} RPM -> {ticks} ticks/op."
                ),
            }
        }
        SpeedFormulaId::MechanicalPress => {
            let tick_speed = lerp(effective_rpm / 512.0, 1.0, 60.0);
            let ticks = (240.0 / tick_speed).ceil().max(1.0);
            MachineSpeedCalculation {
                crafts_per_min: Some(TICKS_PER_MINUTE / ticks),
                ticks_per_craft: Some(ticks),
                source: SpeedSource::ExactFormula,
                stress_per_machine_su: su_per_machine,
                explanation_ru: format!(
                    "Create 6.0.11 Press: цикл 240, шаг {tick_speed:.2} ({effective_rpm} RPM) -> {ticks} тиков/опер."
                ),
                explanation_en: format!(
                    "Create 6.0.11 Press: CYCLE=240, speed {tick_speed:.2} at {effective_rpm} RPM -> {ticks} ticks/op."
                ),
            }
        }
        SpeedFormulaId::MechanicalMixer => {
            let log_term =
                (512.0 / effective_rpm.max(1.0)).log2() * 15.0 * (base_dur / 100.0) + 1.0;
            let ticks = (40.0 + log_term.max(1.0)).ceil();
            MachineSpeedCalculation {
                crafts_per_min: Some(TICKS_PER_MINUTE / ticks),
                ticks_per_craft: Some(ticks),
                source: SpeedSource::ExactFormula,
                stress_per_machine_su: su_per_machine,
                explanation_ru: format!(
                    "Create 6.0.11 Mixer: 40 + log2-время ({effective_rpm} RPM, база {base_dur}) -> {ticks} тиков/опер."
                ),
                explanation_en: format!(
                    "Create 6.0.11 Mixer: 40 + log2 duration ({effective_rpm} RPM, base {base_dur}) -> {ticks} ticks/op."
                ),
            }
        }
        SpeedFormulaId::MechanicalSaw => {
            let speed = clamp(effective_rpm / 24.0, 1.0, 128.0);
            let ticks = (base_dur / speed).ceil().max(1.0);
            MachineSpeedCalculation {
                crafts_per_min: Some(TICKS_PER_MINUTE / ticks),
                ticks_per_craft: Some(ticks),
                source: SpeedSource::ExactFormula,
                stress_per_machine_su: su_per_machine,
                explanation_ru: format!(
                    "Create 6.0.11 Saw: скорость {speed:.2} ({effective_rpm} RPM) -> {ticks} тиков/распил."
                ),
                explanation_en: format!(
                    "Create 6.0.11 Saw: speed {speed:.2} ({effective_rpm} RPM) -> {ticks} ticks/cut."
                ),
            }
        }
        SpeedFormulaId::Deployer => {
            let step = clamp(effective_rpm * 2.0, 8.0, 512.0);
            let ticks = (2500.0 / step).ceil().max(1.0);
            MachineSpeedCalculation {
                crafts_per_min: Some(TICKS_PER_MINUTE / ticks),
                ticks_per_craft: Some(ticks),
                source: SpeedSource::ExactFormula,
                stress_per_machine_su: su_per_machine,
                explanation_ru: format!(
                    "Create 6.0.11 Deployer: фаза 2500 / {step} ({effective_rpm} RPM) -> {ticks} тиков/удар."
                ),
                explanation_en: format!(
                    "Create 6.0.11 Deployer: 2500 / {step} ({effective_rpm} RPM) -> {ticks} ticks/cycle."
                ),
            }
        }
        SpeedFormulaId::Spout | SpeedFormulaId::ItemDrain => {
            let ticks = 20.0;
            MachineSpeedCalculation {
                crafts_per_min: Some(TICKS_PER_MINUTE / ticks),
                ticks_per_craft: Some(ticks),
                source: SpeedSource::ExactFormula,
                stress_per_machine_su: Some(0.0),
                explanation_ru:
                    "Create 6.0.11: фиксированный цикл FILLING_TIME = 20 тиков (60 опер./мин)."
                        .to_owned(),
                explanation_en: "Create 6.0.11: fixed FILLING_TIME = 20 ticks (60 ops/min)."
                    .to_owned(),
            }
        }
        SpeedFormulaId::FixedTicks => {
            let ticks = recipe
                .and_then(|r| r.processing_time_ticks)
                .filter(|d| *d > 0.0)
                .unwrap_or(60.0);
            MachineSpeedCalculation {
                crafts_per_min: Some(TICKS_PER_MINUTE / ticks),
                ticks_per_craft: Some(ticks),
                source: SpeedSource::ExactFormula,
                stress_per_machine_su: Some(0.0),
                explanation_ru: format!(
                    "Время из поля processing_time рецепта: {ticks} тиков ({:.1} опер./мин).",
                    TICKS_PER_MINUTE / ticks
                ),
                explanation_en: format!(
                    "Duration from recipe processing_time: {ticks} ticks ({:.1} ops/min).",
                    TICKS_PER_MINUTE / ticks
                ),
            }
        }
        SpeedFormulaId::CrushingWheel => {
            let batch = clamp(stack_size_assumption.unwrap_or(16.0), 1.0, 64.0);
            let processing_speed = clamp((effective_rpm / 50.0) * 4.0, 0.25, 20.0);
            let ticks_for_batch = ((base_dur - 19.999) / processing_speed).ceil().max(1.0) + 1.0;
            let ticks_per_item = ticks_for_batch / batch;
            MachineSpeedCalculation {
                crafts_per_min: Some(TICKS_PER_MINUTE / ticks_per_item),
                ticks_per_craft: Some(ticks_per_item),
                source: SpeedSource::Assumption,
                stress_per_machine_su: su_per_machine,
                explanation_ru: format!(
                    "Допущение: пачка {batch} шт. при {effective_rpm} RPM (~{ticks_per_item:.2} тиков/предмет)."
                ),
                explanation_en: format!(
                    "Assumption: batch {batch} at {effective_rpm} RPM (~{ticks_per_item:.2} ticks/item)."
                ),
            }
        }
        SpeedFormulaId::EncasedFan => {
            let batch = clamp(stack_size_assumption.unwrap_or(16.0), 1.0, 64.0);
            let batch_mult = ((batch - 1.0) / 16.0).floor() + 1.0;
            let ticks_per_batch = 150.0 * batch_mult;
            let ticks_per_item = ticks_per_batch / batch;
            MachineSpeedCalculation {
                crafts_per_min: Some(TICKS_PER_MINUTE / ticks_per_item),
                ticks_per_craft: Some(ticks_per_item),
                source: SpeedSource::Assumption,
                stress_per_machine_su: su_per_machine,
                explanation_ru: format!(
                    "Допущение: пачка {batch} шт. ({ticks_per_batch} тиков/пачка -> {:.1} опер./мин).",
                    TICKS_PER_MINUTE / ticks_per_item
                ),
                explanation_en: format!(
                    "Assumption: batch {batch} ({ticks_per_batch} ticks/batch -> {:.1} ops/min).",
                    TICKS_PER_MINUTE / ticks_per_item
                ),
            }
        }
        SpeedFormulaId::MechanicalCrafter => {
            let ticks_per_craft = ((500.0 / effective_rpm.max(1.0)) * 9.0).ceil().max(4.0);
            MachineSpeedCalculation {
                crafts_per_min: Some(TICKS_PER_MINUTE / ticks_per_craft),
                ticks_per_craft: Some(ticks_per_craft),
                source: SpeedSource::Assumption,
                stress_per_machine_su: su_per_machine,
                explanation_ru: format!(
                    "Допущение для 3x3 сетки при {effective_rpm} RPM: ~{ticks_per_craft} тиков/крафт."
                ),
                explanation_en: format!(
                    "Assumption for 3x3 grid at {effective_rpm} RPM: ~{ticks_per_craft} ticks/craft."
                ),
            }
        }
        SpeedFormulaId::Unknown => unreachable!(),
    }
}
