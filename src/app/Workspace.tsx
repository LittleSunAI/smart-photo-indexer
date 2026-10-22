import { WorkspaceLayout } from './layout/WorkspaceLayout';

// Compatibility wrapper kept from Step 1. New code should use WorkspaceLayout directly.
export function Workspace() {
  return <WorkspaceLayout />;
}
