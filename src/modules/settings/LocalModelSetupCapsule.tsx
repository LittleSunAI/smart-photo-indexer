import {text as localeText} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import type { LocalAiStatus } from '../localAi/LocalAiStatus';

interface LocalModelSetupCapsuleProps {
  status: LocalAiStatus | null;
  isPreparing: boolean;
  disabled: boolean;
  onPrepareRequest: () => void;
}

function formatProgressPercent(status: LocalAiStatus | null): string {
  return status?.progressPercent !== null && status?.progressPercent !== undefined ? `${status.progressPercent}%` : '';
}

function getCapsuleLabel(status: LocalAiStatus | null, isPreparing: boolean): string {
  const percent = formatProgressPercent(status);

  if (status?.modelStatus === 'pulling') return percent ? localeText("settings", "ui.LocalModelSetupCapsule.d5cf2276132ba4b1", {"value1": (percent)}) : localeText("settings", "ui.LocalModelSetupCapsule.ec3e280a5f40dc50");
  if (isPreparing || status?.runtimeStatus === 'installing') return localeText("settings", "ui.LocalModelSetupCapsule.51bcc47b4709b457");
  if (status?.runtimeStatus === 'starting') return localeText("settings", "ui.LocalModelSetupCapsule.fde30181bbae495e");
  if (status?.modelStatus === 'verifying') return localeText("settings", "ui.LocalModelSetupCapsule.53c071fc59ea9e19");
  if (status?.runtimeStatus === 'running' && status.modelStatus === 'ready') return localeText("settings", "ui.LocalModelSetupCapsule.6809fe65bfc60f70");
  if (status?.runtimeStatus === 'error' || status?.modelStatus === 'error') return localeText("settings", "ui.LocalModelSetupCapsule.0c1c8520baefbba8");
  return localeText("settings", "ui.LocalModelSetupCapsule.82e70e6f07dea006");
}

function getCapsuleClassName(status: LocalAiStatus | null, isPreparing: boolean): string {
  const classes = ['local-setup-capsule'];
  const isBusy = isPreparing
    || status?.runtimeStatus === 'installing'
    || status?.runtimeStatus === 'starting'
    || status?.modelStatus === 'pulling'
    || status?.modelStatus === 'verifying';

  if (isBusy) {
    classes.push('local-setup-capsule--busy');
  } else if (status?.runtimeStatus === 'running' && status.modelStatus === 'ready') {
    classes.push('local-setup-capsule--ready');
  } else if (status?.runtimeStatus === 'error' || status?.modelStatus === 'error') {
    classes.push('local-setup-capsule--error');
  } else {
    classes.push('local-setup-capsule--idle');
  }

  return classes.join(' ');
}

function getCapsuleTitle(status: LocalAiStatus | null): string {
  if (!status) return localeText("settings", "ui.LocalModelSetupCapsule.1e683a95291f1ed1");
  const percent = formatProgressPercent(status);
  const progress = percent ? ` ${percent}` : '';
  const details = status.errorMessage ? ` ${status.errorMessage}` : '';
  return `${status.statusText}${progress}${details}`.trim();
}

export function LocalModelSetupCapsule({ status, isPreparing, disabled, onPrepareRequest }: LocalModelSetupCapsuleProps) {
  const isReady = status?.runtimeStatus === 'running' && status.modelStatus === 'ready';
  const isBusy = isPreparing
    || status?.runtimeStatus === 'installing'
    || status?.runtimeStatus === 'starting'
    || status?.modelStatus === 'pulling'
    || status?.modelStatus === 'verifying';

  const handlePrepareClick = () => {
    onPrepareRequest();
  };

  return (
    <Hint message={{id:"local.setup-capsule"}}><button
      type="button"
      className={getCapsuleClassName(status, isPreparing)}
      disabled={disabled || isBusy || isReady}
      title={getCapsuleTitle(status)}
      onClick={handlePrepareClick}
    >
      {getCapsuleLabel(status, isPreparing)}
    </button></Hint>
  );
}
