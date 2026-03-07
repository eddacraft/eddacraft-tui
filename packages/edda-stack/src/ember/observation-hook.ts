import type { CandidateProposal } from '../contracts/ember-proposal.js';
import type { IStackEventBus } from '../contracts/events.js';

export interface ObservationSessionProcessor {
  processSession(sessionId: string): Promise<CandidateProposal[]>;
}

export interface ObservationHookDeps {
  candidateService: ObservationSessionProcessor;
  eventBus?: IStackEventBus;
  onError?: (error: Error, sessionId: string) => void | Promise<void>;
}

export class ObservationHook {
  private _unsubscribe?: () => void;

  constructor(private readonly deps: ObservationHookDeps) {}

  start(): void {
    if (this._unsubscribe || !this.deps.eventBus) {
      return;
    }

    this._unsubscribe = this.deps.eventBus.subscribe('session_completed', async (event) => {
      try {
        await this.deps.candidateService.processSession(event.payload.session_id);
      } catch (error) {
        await this.handleProcessingError(event.payload.session_id, error);
      }
    });
  }

  stop(): void {
    this._unsubscribe?.();
    this._unsubscribe = undefined;
  }

  async processSession(sessionId: string): Promise<CandidateProposal[]> {
    return this.deps.candidateService.processSession(sessionId);
  }

  isActive(): boolean {
    return this._unsubscribe !== undefined;
  }

  private async handleProcessingError(sessionId: string, error: unknown): Promise<void> {
    const resolvedError = error instanceof Error ? error : new Error(String(error));

    if (this.deps.onError) {
      await this.deps.onError(resolvedError, sessionId);
      return;
    }

    console.error(`ObservationHook failed to process session ${sessionId}`, resolvedError);
  }
}
