// ── Agent Interface for Canvas Manipulation ──
// Provides a unified method for AI agents to manipulate canvas state
// without requiring a page refresh. Supports animated transitions.

import type { AgentAction } from "./types";
import type { KnowledgeCanvasHandle } from "./KnowledgeCanvas";

export interface AgentCommand {
  actions: AgentAction[];
  /** Optional metadata for tracking agent operation provenance. */
  meta?: {
    agentId?: string;
    taskId?: string;
    reason?: string;
  };
}

/**
 * Apply a series of agent actions to a canvas with animated transitions.
 *
 * @param canvasRef - Reference to a KnowledgeCanvas component
 * @param commands - One or more agent command batches to execute
 */
export async function updateGraphByAgent(
  canvasRef: React.RefObject<KnowledgeCanvasHandle | null>,
  commands: AgentCommand | AgentCommand[],
): Promise<void> {
  const cmds = Array.isArray(commands) ? commands : [commands];

  for (const cmd of cmds) {
    if (cmd.actions.length === 0) continue;

    if (cmd.meta?.reason) {
      console.log(
        `[AgentInterface] Agent ${cmd.meta.agentId ?? "unknown"} executing ${cmd.actions.length} action(s): ${cmd.meta.reason}`,
      );
    }

    // Apply actions to the canvas
    if (canvasRef.current) {
      await canvasRef.current.applyAgentActions(cmd.actions);
    }
  }
}

/**
 * Singleton helper to queue agent actions even before a canvas is mounted.
 * The queue is flushed once the canvas ref is available.
 */
export class AgentActionQueue {
  private static queue: AgentCommand[] = [];
  private static canvasRef: React.RefObject<KnowledgeCanvasHandle | null> | null = null;

  static bindCanvas(ref: React.RefObject<KnowledgeCanvasHandle | null>) {
    this.canvasRef = ref;
    this.flush();
  }

  static enqueue(command: AgentCommand) {
    this.queue.push(command);
    if (this.canvasRef?.current) {
      this.flush();
    }
  }

  static async flush() {
    if (!this.canvasRef?.current || this.queue.length === 0) return;
    const pending = [...this.queue];
    this.queue = [];
    for (const cmd of pending) {
      // Each action sequentially with animation delay
      for (const action of cmd.actions) {
        await this.canvasRef.current!.applyAgentActions([action]);
        await new Promise((r) => setTimeout(r, 200)); // Animation delay
      }
    }
  }

  static clear() {
    this.queue = [];
  }
}
