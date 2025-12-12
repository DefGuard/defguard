import { createFileRoute, Outlet } from '@tanstack/react-router';
import { DisplayListModal } from '../shared/components/DisplayListModal/DisplayListModal';
import { SelectionModal } from '../shared/components/modals/SelectionModal/SelectionModal';
import { AppConfigProvider } from '../shared/providers/AppConfigProvider';

export const Route = createFileRoute('/_authorized')({
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <AppConfigProvider>
      <Outlet />
      <DisplayListModal />
      <SelectionModal />
    </AppConfigProvider>
  );
}
