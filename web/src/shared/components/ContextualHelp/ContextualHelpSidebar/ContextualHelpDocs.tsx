import { m } from '../../../../paraglide/messages';
import { Icon, IconKind } from '../../../defguard-ui/components/Icon';
import { ThemeVariable } from '../../../defguard-ui/types';
import type { ContextualHelpDoc } from '../types';

type Props = {
  docs: ContextualHelpDoc[];
};

export const ContextualHelpDocs = ({ docs }: Props) => (
  <div className="contextual-help-section">
    <div className="header">
      <Icon icon={IconKind.File} />
      <span className="title">{m.cmp_contextual_help_related_docs()}</span>
    </div>
    <div className="docs-card">
      {docs.map((doc, i) => (
        <a
          key={i}
          className="docs-item"
          href={doc.url}
          target="_blank"
          rel="noopener noreferrer"
        >
          <div className="badge">
            <Icon
              icon={IconKind.ActivityNotes}
              size={20}
              staticColor={ThemeVariable.FgAction}
            />
          </div>
          <div className="link">
            <span>{doc.title}</span>
            <Icon icon={IconKind.OpenInNewWindow} staticColor={ThemeVariable.FgAction} />
          </div>
        </a>
      ))}
    </div>
  </div>
);
