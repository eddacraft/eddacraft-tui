import type { CandidateProposal } from '../contracts/ember-proposal.js';
import type { IStackEventBus } from '../contracts/events.js';
import { CandidateService } from './candidate-service.js';

export interface ObservationHookDeps {
  candidateService: CandidateService;
  eventBus?: IStackEventBus;
}

export class ObservationHook {
  private _unsubscribe?: () => void;

  constructor(private readonly deps: ObservationHookDeps) {}

  start(): void {
    if (this._unsubscribe || !this.deps.eventBus) {
      return;
    }

    this._unsubscribe = this.deps.eventBus.subscribe('session_completed', async (event) => {
      await this.deps.candidateService.processSession(event.payload.session_id);
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
}
