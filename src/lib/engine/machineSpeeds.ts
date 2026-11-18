import type { MachineDef, MachineSpeedMode, RecipeDef } from '../types/atlas';

export interface SpeedComputationResult {
  craftsPerMin: number | null;
  ticksPerCraft: number | null;
  source: 'exact-formula' | 'manual' | 'assumption' | 'unknown';
  explanationRu: string;
  explanationEn: string;
}

const TICKS_PER_MINUTE = 1200; // 20 ticks/sec * 60 sec

/**
 * Computes machine throughput based strictly on verified Create 6.0.11 (mc1.21.1)
 * Java block entity tick formulas, user manual rates, or explicitly tagged assumptions.
 * Never fabricates unknown speeds.
 */
export function computeMachineThroughput(params: {
  machine: MachineDef | undefined;
  recipe: RecipeDef | undefined;
  rpm?: number;
  speedMode?: MachineSpeedMode;
  manualCraftsPerMin?: number;
  stackSizeAssumption?: number;
}): SpeedComputationResult {
  const { machine, recipe, rpm, speedMode, manualCraftsPerMin, stackSizeAssumption } = params;

  if (speedMode === 'manual') {
    if (manualCraftsPerMin !== undefined && Number.isFinite(manualCraftsPerMin) && manualCraftsPerMin > 0) {
      return {
        craftsPerMin: manualCraftsPerMin,
        ticksPerCraft: TICKS_PER_MINUTE / manualCraftsPerMin,
        source: 'manual',
        explanationRu: `Задано вручную пользователем: ${manualCraftsPerMin.toFixed(2)} опер./мин`,
        explanationEn: `User-specified manual throughput: ${manualCraftsPerMin.toFixed(2)} ops/min`,
      };
    }
    return {
      craftsPerMin: null,
      ticksPerCraft: null,
      source: 'unknown',
      explanationRu: 'Выбран ручной режим скорости, но темп (опер./мин) не задан.',
      explanationEn: 'Manual speed mode selected, but ops/min is not set.',
    };
  }

  if (speedMode === 'unknown' || !machine) {
    return {
      craftsPerMin: null,
      ticksPerCraft: null,
      source: 'unknown',
      explanationRu: 'Скорость машины неизвестна — укажите RPM или ручной темп в свойствах узла.',
      explanationEn: 'Machine speed is unknown — specify RPM or manual rate in node inspector.',
    };
  }

  const effectiveRpm = rpm ?? machine.defaultRpm ?? null;
  if (machine.supportsRpm && (effectiveRpm === null || !Number.isFinite(effectiveRpm) || effectiveRpm <= 0)) {
    return {
      craftsPerMin: null,
      ticksPerCraft: null,
      source: 'unknown',
      explanationRu: 'Для расчёта требуется скорость вращения (RPM > 0) или ручной ввод темпа.',
      explanationEn: 'Requires rotation speed (RPM > 0) or manual rate input.',
    };
  }

  const absRpm = Math.min(Math.max(Math.abs(effectiveRpm ?? 0), 1), machine.maxRpm || 256);
  const baseDuration = recipe?.processingTimeTicks ?? null;

  switch (machine.speedFormulaId) {
    case 'millstone': {
      if (baseDuration === null || baseDuration <= 0) {
        return unknownDuration(machine.nameRu, machine.nameEn);
      }
      // MillstoneBlockEntity.java: timer -= clamp(floor(|speed|/16), 1, 512)
      const step = Math.min(512, Math.max(1, Math.floor(absRpm / 16)));
      const ticks = Math.ceil(baseDuration / step) + 1;
      const cpm = TICKS_PER_MINUTE / ticks;
      return {
        craftsPerMin: cpm,
        ticksPerCraft: ticks,
        source: 'exact-formula',
        explanationRu: `Жёрнов (${absRpm} RPM): шаг ${step}/тик при длит. ${baseDuration} тиков ≈ ${ticks} тик./опер.`,
        explanationEn: `Millstone (${absRpm} RPM): step ${step}/tick over ${baseDuration} ticks ≈ ${ticks} ticks/op.`,
      };
    }

    case 'mechanical_press': {
      // PressingBehaviour.java: CYCLE = 240, runningTickSpeed = floor(lerp(clamp(|rpm|/512, 0, 1), 1, 60))
      const t = Math.min(1, Math.max(0, absRpm / 512));
      const tickSpeed = Math.max(1, Math.floor(1 + t * (60 - 1)));
      const ticks = Math.ceil(240 / tickSpeed) + 1;
      const cpm = TICKS_PER_MINUTE / ticks;
      return {
        craftsPerMin: cpm,
        ticksPerCraft: ticks,
        source: 'exact-formula',
        explanationRu: `Мех. пресс (${absRpm} RPM): цикл 240 со скоростью ${tickSpeed}/тик ≈ ${ticks} тик./опер.`,
        explanationEn: `Mechanical Press (${absRpm} RPM): cycle 240 at ${tickSpeed}/tick ≈ ${ticks} ticks/op.`,
      };
    }

    case 'mechanical_mixer': {
      // MechanicalMixerBlockEntity.java:
      // recipeSpeed = duration ? duration/100 : 1
      // processingTicks = max(floor(log2(floor(512 / speed))) * ceil(recipeSpeed * 15) + 1, 1)
      // + ~40 ticks for lowering/raising head
      const recipeSpeed = baseDuration && baseDuration > 0 ? baseDuration / 100 : 1;
      const ratio = Math.max(1, Math.floor(512 / absRpm));
      const log2Ratio = Math.floor(Math.log2(ratio));
      const procTicks = Math.max(1, log2Ratio * Math.ceil(recipeSpeed * 15) + 1);
      const ticks = 40 + procTicks;
      const cpm = TICKS_PER_MINUTE / ticks;
      return {
        craftsPerMin: cpm,
        ticksPerCraft: ticks,
        source: 'exact-formula',
        explanationRu: `Мех. смешиватель (${absRpm} RPM): 40 + ${procTicks} тиков = ${ticks} тик./опер.`,
        explanationEn: `Mechanical Mixer (${absRpm} RPM): 40 + ${procTicks} ticks = ${ticks} ticks/op.`,
      };
    }

    case 'mechanical_saw': {
      // SawBlockEntity.java: recipeDuration = 20 (default cutting/recipe), processingSpeed = clamp(|rpm|/24, 1, 128)
      const dur = baseDuration && baseDuration > 0 ? baseDuration : 20;
      const procSpeed = Math.min(128, Math.max(1, absRpm / 24));
      const ticks = Math.ceil(dur / procSpeed) + 1;
      const cpm = TICKS_PER_MINUTE / ticks;
      return {
        craftsPerMin: cpm,
        ticksPerCraft: ticks,
        source: 'exact-formula',
        explanationRu: `Мех. пила (${absRpm} RPM): длит. ${dur}, скорость ${procSpeed.toFixed(2)}/тик ≈ ${ticks} тик./опер.`,
        explanationEn: `Mechanical Saw (${absRpm} RPM): duration ${dur}, speed ${procSpeed.toFixed(2)}/tick ≈ ${ticks} ticks/op.`,
      };
    }

    case 'deployer': {
      // DeployerBlockEntity.java: timerSpeed = clamp(floor(|rpm|*2), 8, 512), 1000 + 1000 + 500
      const timerSpeed = Math.min(512, Math.max(8, Math.floor(absRpm * 2)));
      const ticks = Math.ceil(1000 / timerSpeed) * 2 + Math.ceil(500 / timerSpeed) + 3;
      const cpm = TICKS_PER_MINUTE / ticks;
      return {
        craftsPerMin: cpm,
        ticksPerCraft: ticks,
        source: 'exact-formula',
        explanationRu: `Автон. активатор (${absRpm} RPM): шаг ${timerSpeed}, цикл 2500 ≈ ${ticks} тик./опер.`,
        explanationEn: `Deployer (${absRpm} RPM): speed ${timerSpeed}, 2500 phases ≈ ${ticks} ticks/op.`,
      };
    }

    case 'spout':
    case 'item_drain': {
      // SpoutBlockEntity.java / ItemDrainBlockEntity.java: FILLING_TIME = 20 ticks
      const ticks = 20;
      const cpm = TICKS_PER_MINUTE / ticks;
      return {
        craftsPerMin: cpm,
        ticksPerCraft: ticks,
        source: 'exact-formula',
        explanationRu: `${machine.nameRu}: фиксированная фаза FILLING_TIME = 20 тиков (60 опер./мин без учёта задержки ленты).`,
        explanationEn: `${machine.nameEn}: fixed FILLING_TIME = 20 ticks (60 ops/min excl. belt travel).`,
      };
    }

    case 'crushing_wheel': {
      if (baseDuration === null || baseDuration <= 0) {
        return unknownDuration(machine.nameRu, machine.nameEn);
      }
      // CrushingWheelControllerBlockEntity.java:
      // speed = crushingspeed * 4 = (|rpm|/50)*4
      // processingSpeed = clamp(speed / max(1, log2(stackCount)), 0.25, 20)
      // Note: stackCount depends on player setup (belt vs chute), so marked as assumption!
      const stack = Math.min(64, Math.max(1, stackSizeAssumption ?? 1));
      const logDiv = stack <= 1 ? 1 : Math.log2(stack);
      const rawSpeed = (absRpm / 50) * 4;
      const procSpeed = Math.min(20, Math.max(0.25, rawSpeed / logDiv));
      const ticks = Math.ceil(baseDuration / procSpeed);
      const itemsPerBatch = stack;
      const cpm = (TICKS_PER_MINUTE / ticks) * itemsPerBatch;
      return {
        craftsPerMin: cpm,
        ticksPerCraft: ticks / itemsPerBatch,
        source: 'assumption',
        explanationRu: `Допущение (размер пачки = ${stack}): дроб. колёса (${absRpm} RPM), ${ticks} тик. на пачку -> ${cpm.toFixed(1)} предм./мин.`,
        explanationEn: `Assumption (batch size = ${stack}): Crushing Wheels (${absRpm} RPM), ${ticks} ticks/batch -> ${cpm.toFixed(1)} items/min.`,
      };
    }

    case 'encased_fan': {
      // FanProcessing.java & CKinetics.java: fanProcessingTime = 150 ticks * ((stack-1)/16 + 1)
      const stack = Math.min(64, Math.max(1, stackSizeAssumption ?? 16));
      const mult = Math.floor((stack - 1) / 16) + 1;
      const ticks = 150 * mult + 1;
      const cpm = (TICKS_PER_MINUTE / ticks) * stack;
      return {
        craftsPerMin: cpm,
        ticksPerCraft: ticks / stack,
        source: 'assumption',
        explanationRu: `Допущение (пачка ${stack} шт. перед вентилятором): ${ticks} тиков на пачку -> ${cpm.toFixed(1)} предм./мин (RPM влияет на дальность потока, не на время).`,
        explanationEn: `Assumption (batch ${stack} items in airstream): ${ticks} ticks/batch -> ${cpm.toFixed(1)} items/min (RPM sets reach, not processing time).`,
      };
    }

    case 'mechanical_crafter': {
      // MechanicalCrafterBlockEntity.java: countDownSpeed = clamp(floor(|rpm|), 4, 250); crafting phase ~2000 + transfer ~1000
      const speed = Math.min(250, Math.max(4, Math.floor(absRpm)));
      const ticks = Math.ceil(2000 / speed) + Math.ceil(1000 / speed) + 2;
      const cpm = TICKS_PER_MINUTE / ticks;
      return {
        craftsPerMin: cpm,
        ticksPerCraft: ticks,
        source: 'assumption',
        explanationRu: `Допущение (цикл сборщика при ${absRpm} RPM): скорость ${speed}, ≈ ${ticks} тик./опер. (зависит от числа ячеек в сетке).`,
        explanationEn: `Assumption (crafter cycle at ${absRpm} RPM): speed ${speed}, ≈ ${ticks} ticks/op (depends on grid chain length).`,
      };
    }

    case 'fixed_ticks': {
      if (baseDuration && baseDuration > 0) {
        const cpm = TICKS_PER_MINUTE / baseDuration;
        return {
          craftsPerMin: cpm,
          ticksPerCraft: baseDuration,
          source: 'exact-formula',
          explanationRu: `Фиксированное время рецепта: ${baseDuration} тиков (${cpm.toFixed(2)} опер./мин).`,
          explanationEn: `Fixed recipe time: ${baseDuration} ticks (${cpm.toFixed(2)} ops/min).`,
        };
      }
      return unknownDuration(machine.nameRu, machine.nameEn);
    }

    case 'unknown':
    default:
      return {
        craftsPerMin: null,
        ticksPerCraft: null,
        source: 'unknown',
        explanationRu: `Для машины «${machine.nameRu}» скорость не подтверждена в источнике. Задайте темп вручную.`,
        explanationEn: `Speed for "${machine.nameEn}" is not verified from upstream source. Enter manual rate.`,
      };
  }
}

function unknownDuration(nameRu: string, nameEn: string): SpeedComputationResult {
  return {
    craftsPerMin: null,
    ticksPerCraft: null,
    source: 'unknown',
    explanationRu: `Для «${nameRu}» не задано время обработки рецепта (processing_time). Укажите вручную.`,
    explanationEn: `Missing recipe processing_time for "${nameEn}". Enter manual rate.`,
  };
}
