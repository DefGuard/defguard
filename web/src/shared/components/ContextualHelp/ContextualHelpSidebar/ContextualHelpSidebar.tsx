import { isPresent } from '../../../defguard-ui/utils/isPresent';
import { useContextualHelp } from '../hooks';
import type { ContextualHelpKey } from '../types';
import { ContextualHelpBestPractices } from './ContextualHelpBestPractices';
import { ContextualHelpDocs } from './ContextualHelpDocs';
import { ContextualHelpFaqs } from './ContextualHelpFaqs';
import './ContextualHelpSidebar.scss';

type Props = {
  pageKey: ContextualHelpKey;
};

export const ContextualHelpSidebar = ({ pageKey }: Props) => {
  const page = useContextualHelp(pageKey);

  if (!page) return null;

  const { faqs, relatedDocs, bestPractices } = page;
  const hasContent = isPresent(faqs) || isPresent(relatedDocs) || bestPractices;

  if (!hasContent) return null;

  return (
    <div className="contextual-help">
      {isPresent(faqs) && <ContextualHelpFaqs faqs={faqs} />}
      {isPresent(relatedDocs) && <ContextualHelpDocs docs={relatedDocs} />}
      {bestPractices && <ContextualHelpBestPractices content={bestPractices} />}
    </div>
  );
};
