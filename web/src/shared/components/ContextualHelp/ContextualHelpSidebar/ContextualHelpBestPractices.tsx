import { m } from '../../../../paraglide/messages';
import { Icon, IconKind } from '../../../defguard-ui/components/Icon';
import { RenderMarkdown } from '../../../defguard-ui/components/RenderMarkdown/RenderMarkdown';

type Props = {
  content: string;
};

export const ContextualHelpBestPractices = ({ content }: Props) => (
  <div className="contextual-help-section">
    <div className="header">
      <Icon icon={IconKind.LightBulb} />
      <span className="title">{m.cmp_contextual_help_best_practices()}</span>
    </div>
    <div className="best-practices-card">
      <RenderMarkdown content={content} />
    </div>
  </div>
);
