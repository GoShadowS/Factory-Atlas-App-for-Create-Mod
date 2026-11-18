import type { FactoryProject } from '../types/atlas';

export interface UndoStack {
  past: FactoryProject[];
  future: FactoryProject[];
}

const MAX_UNDO_DEPTH = 60;

export function pushHistory(stack: UndoStack, snapshot: FactoryProject): UndoStack {
  const cloned = structuredClone(snapshot);
  const nextPast = [...stack.past, cloned];
  if (nextPast.length > MAX_UNDO_DEPTH) {
    nextPast.shift();
  }
  return {
    past: nextPast,
    future: [],
  };
}

export function performUndo(
  stack: UndoStack,
  current: FactoryProject
): { stack: UndoStack; next: FactoryProject } | null {
  if (stack.past.length === 0) return null;
  const previous = stack.past[stack.past.length - 1];
  return {
    stack: {
      past: stack.past.slice(0, -1),
      future: [structuredClone(current), ...stack.future].slice(0, MAX_UNDO_DEPTH),
    },
    next: structuredClone(previous),
  };
}

export function performRedo(
  stack: UndoStack,
  current: FactoryProject
): { stack: UndoStack; next: FactoryProject } | null {
  if (stack.future.length === 0) return null;
  const restored = stack.future[0];
  return {
    stack: {
      past: [...stack.past, structuredClone(current)].slice(-MAX_UNDO_DEPTH),
      future: stack.future.slice(1),
    },
    next: structuredClone(restored),
  };
}
