import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/_authorized/_default/settings/ca')({
  component: RouteComponent,
})

function RouteComponent() {
  return <div>Hello "/_authorized/_default/settings/ca"!</div>
}
