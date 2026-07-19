import type { components } from '@/api/generated/openapi';
import { MetricCard, type MetricCardProps } from '@/components/primitives/metric-card';

type Overview = components['schemas']['ProtectionOverview'];
type CardFact = Pick<MetricCardProps, 'detail' | 'state' | 'value'>;

const observedDateFormatter = new Intl.DateTimeFormat('en-GB', {
  day: '2-digit',
  month: 'short',
  timeZone: 'UTC',
  year: 'numeric',
});
const observedTimeFormatter = new Intl.DateTimeFormat('en-GB', {
  hour: '2-digit',
  hour12: false,
  minute: '2-digit',
  timeZone: 'UTC',
});

function sentenceCase(value: string) {
  const words = value.replaceAll('-', ' ');
  return words.charAt(0).toUpperCase() + words.slice(1);
}

function protectionFact(overview: Overview): CardFact {
  const saveTime = overview.save_time;
  if (!saveTime) {
    return { value: 'Not observed', detail: 'Live state unavailable', state: 'unavailable' };
  }
  return {
    value: saveTime.active ? 'Active' : 'Not observed',
    detail: sentenceCase(saveTime.state),
    state: 'complete',
  };
}

function latestGateFact(overview: Overview): CardFact {
  const latest = overview.latest_run;
  if (!latest) {
    return { value: 'Unavailable', detail: 'No gate run recorded', state: 'unavailable' };
  }
  return {
    value: latest.score === null ? 'No score' : `${Math.round(latest.score)}/100`,
    detail: latest.label,
    state: latest.score === null ? 'partial' : 'complete',
  };
}

function warningsFact(overview: Overview): CardFact {
  if (overview.warnings_state === 'complete') {
    return {
      value: String(overview.warnings.length),
      detail: 'Complete warning resource',
      state: 'complete',
    };
  }
  if (overview.warnings_state === 'partial') {
    return {
      value: overview.warnings.length > 0 ? `${overview.warnings.length} shown` : 'Partial',
      detail: 'Partial warning history',
      state: 'partial',
    };
  }
  return {
    value: 'Unavailable',
    detail: 'Warning history unavailable',
    state: 'unavailable',
  };
}

function assuranceFact(overview: Overview): CardFact {
  const assurance = overview.assurance;
  if (!assurance) {
    return {
      value: 'Unavailable',
      detail: 'Workspace assurance unavailable',
      state: 'unavailable',
    };
  }
  const scanned = assurance.scanned_files;
  const total = assurance.total_files;
  if (scanned !== null && total !== null && total > 0) {
    return {
      value: `${Math.round((scanned / total) * 100)}%`,
      detail: `${scanned} of ${total} files · ${sentenceCase(assurance.state)}`,
      state: scanned === total ? 'complete' : 'partial',
    };
  }
  return {
    value: sentenceCase(assurance.state),
    detail: `Generation ${assurance.generation}`,
    state: 'partial',
  };
}

function freshnessFact(overview: Overview): CardFact {
  if (!overview.observed_at_unix) {
    return { value: 'Not observed', detail: 'Live timestamp unavailable', state: 'unavailable' };
  }
  const observedAt = new Date(overview.observed_at_unix * 1000);
  const date = observedDateFormatter.format(observedAt);
  const time = observedTimeFormatter.format(observedAt);
  return { value: date, detail: `${time} UTC`, state: 'complete' };
}

export function CurrentHealthCards({ overview }: { overview: Overview }) {
  const cards = [
    { label: 'Save-time protection', ...protectionFact(overview) },
    { label: 'Latest gate', ...latestGateFact(overview) },
    { label: 'Active warnings', ...warningsFact(overview) },
    { label: 'Workspace assurance', ...assuranceFact(overview) },
    { label: 'Evidence freshness', ...freshnessFact(overview) },
  ] satisfies MetricCardProps[];

  return (
    <section aria-label="Current workspace health" className="current-health-cards">
      {cards.map((card) => (
        <MetricCard key={card.label} {...card} />
      ))}
    </section>
  );
}
