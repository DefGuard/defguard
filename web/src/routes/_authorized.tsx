import { createFileRoute, Outlet } from '@tanstack/react-router';
import { DisplayListModal } from '../shared/components/DisplayListModal/DisplayListModal';
import { SelectionModal } from '../shared/components/modals/SelectionModal/SelectionModal';

export const Route = createFileRoute('/_authorized')({
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <>
      <Outlet />
      <DisplayListModal />
      <SelectionModal />
    </>
  );
}
