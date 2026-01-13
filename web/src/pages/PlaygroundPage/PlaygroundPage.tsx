import { m } from '../../paraglide/messages';
import { CodeCard } from '../../shared/defguard-ui/components/CodeCard/CodeCard';
import './style.scss';

export const PlaygroundPage = () => {
  return (
    <div id="playground-page">
      <CodeCard title="Code section title" value={m.test_placeholder_extreme()} />
    </div>
  );
};
