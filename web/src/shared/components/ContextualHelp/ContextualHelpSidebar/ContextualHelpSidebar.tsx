import clsx from 'clsx';
import { useState } from 'react';
import { m } from '../../../../paraglide/messages';
import { Fold } from '../../../defguard-ui/components/Fold/Fold';
import { Icon, IconKind } from '../../../defguard-ui/components/Icon';
import { RenderMarkdown } from '../../../defguard-ui/components/RenderMarkdown/RenderMarkdown';
import { useContextualHelp } from '../hooks';
import type { ContextualHelpKey } from '../types';
import './ContextualHelpSidebar.scss';

type Props = {
  pageKey: ContextualHelpKey;
};

type FaqItemProps = {
  question: string;
  answer: string;
};

const FaqItem = ({ question, answer }: FaqItemProps) => {
  const [open, setOpen] = useState(false);

  return (
    <div
      className="faq-item"
      onClick={() => setOpen((v) => !v)}
      role="button"
      tabIndex={0}
    >
      <Icon
        icon={open ? IconKind.MinusCircle : IconKind.PlusCircle}
        className={clsx('faq-item-icon', { open })}
      />
      <div className="faq-item-content">
        <p className={clsx('faq-question', { open })}>{question}</p>
        <Fold open={open}>
          <p className="faq-answer">{answer}</p>
        </Fold>
      </div>
    </div>
  );
};

export const ContextualHelpSidebar = ({ pageKey }: Props) => {
  const page = useContextualHelp(pageKey);

  if (!page) return null;

  const { faqs, relatedDocs, bestPractices } = page;
  const hasContent =
    (faqs && faqs.length > 0) || (relatedDocs && relatedDocs.length > 0) || bestPractices;

  if (!hasContent) return null;

  return (
    <div className="contextual-help">
      {faqs && faqs.length > 0 && (
        <div className="contextual-help-section">
          <div className="section-header">
            <Icon icon={IconKind.Chat} />
            <span className="section-title">{m.cmp_contextual_help_faq()}</span>
          </div>
          <div className="faq-card">
            {faqs.map((faq, i) => (
              <FaqItem key={i} question={faq.question} answer={faq.answer} />
            ))}
          </div>
        </div>
      )}

      {relatedDocs && relatedDocs.length > 0 && (
        <div className="contextual-help-section">
          <div className="section-header">
            <Icon icon={IconKind.File} />
            <span className="section-title">{m.cmp_contextual_help_related_docs()}</span>
          </div>
          <div className="docs-card">
            {relatedDocs.map((doc, i) => (
              <a
                key={i}
                className="docs-item"
                href={doc.url}
                target="_blank"
                rel="noopener noreferrer"
              >
                <div className="docs-icon-badge">
                  <Icon icon={IconKind.ActivityNotes} />
                </div>
                <div className="docs-link">
                  <span>{doc.title}</span>
                  <Icon icon={IconKind.OpenInNewWindow} className="docs-external-icon" />
                </div>
              </a>
            ))}
          </div>
        </div>
      )}

      {bestPractices && (
        <div className="contextual-help-section">
          <div className="section-header">
            <Icon icon={IconKind.LightBulb} />
            <span className="section-title">
              {m.cmp_contextual_help_best_practices()}
            </span>
          </div>
          <div className="practices-card">
            <RenderMarkdown content={bestPractices} />
          </div>
        </div>
      )}
    </div>
  );
};
