import {text as localeText} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import type { OpenRouterCheckResult } from './OpenRouterCheckService';

interface OpenRouterCheckCapsuleProps {
  status: OpenRouterCheckResult | null;
  isChecking: boolean;
  disabled: boolean;
  onCheckRequest: () => void;
}

function getCapsuleLabel(status: OpenRouterCheckResult | null, isChecking: boolean): string {
  if (isChecking || status?.status === 'checking') return localeText("main", "ui.OpenRouterCheckCapsule.e03cb95870529baa");
  if (status?.status === 'ready') return status.limitRemaining !== null ? `✓ ${status.limitRemaining.toFixed(1)}` : localeText("main", "ui.OpenRouterCheckCapsule.807952d3320e28cc");
  if (status?.status === 'missingKey') return localeText("main", "ui.OpenRouterCheckCapsule.1e732868d975c6f8");
  if (status?.status === 'invalidKey' || status?.status === 'error') return localeText("main", "ui.OpenRouterCheckCapsule.b496da5cd5dd5a69");
  return localeText("main", "ui.OpenRouterCheckCapsule.c6b3ad86b0d5b135");
}

function getCapsuleClassName(status: OpenRouterCheckResult | null, isChecking: boolean): string {
  const classes = ['openrouter-check-capsule'];

  if (isChecking || status?.status === 'checking') {
    classes.push('openrouter-check-capsule--busy');
  } else if (status?.status === 'ready') {
    classes.push('openrouter-check-capsule--ready');
  } else if (status?.status === 'missingKey' || status?.status === 'invalidKey' || status?.status === 'error') {
    classes.push('openrouter-check-capsule--error');
  } else {
    classes.push('openrouter-check-capsule--idle');
  }

  return classes.join(' ');
}

function getCapsuleTitle(status: OpenRouterCheckResult | null): string {
  return status?.message ?? localeText("main", "ui.OpenRouterCheckCapsule.dd402449b85fce24");
}

export function OpenRouterCheckCapsule({ status, isChecking, disabled, onCheckRequest }: OpenRouterCheckCapsuleProps) {
  const isBusy = isChecking || status?.status === 'checking';

  return (
    <Hint message={{id:"cloud.check-capsule"}}><button
      type="button"
      className={getCapsuleClassName(status, isChecking)}
      disabled={disabled || isBusy}
      title={getCapsuleTitle(status)}
      onClick={onCheckRequest}
    >
      {getCapsuleLabel(status, isChecking)}
    </button></Hint>
  );
}
