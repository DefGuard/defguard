import clsx from 'clsx';
import { useState } from 'react';
import { m } from '../../../../paraglide/messages';
import { Fold } from '../../../defguard-ui/components/Fold/Fold';
import { Icon, IconKind } from '../../../defguard-ui/components/Icon';
import { ThemeVariable } from '../../../defguard-ui/types';
import type { ContextualHelpFaq } from '../types';

type FaqItemProps = {
  question: string;
  answer: string;
};

const FaqItem = ({ question, answer }: FaqItemProps) => {
  const [open, setOpen] = useState(false);

  return (
    <div className="faq-item" onClick={() => setOpen((v) => !v)} role="button">
      <Icon
        icon={open ? IconKind.MinusCircle : IconKind.PlusCircle}
        staticColor={open ? ThemeVariable.FgNeutral : ThemeVariable.FgAction}
      />
      <div className="content">
        <p className={clsx('question', { open })}>{question}</p>
        <Fold open={open}>
          <p className="answer">{answer}</p>
        </Fold>
      </div>
    </div>
  );
};

type Props = {
  faqs: ContextualHelpFaq[];
};

export const ContextualHelpFaqs = ({ faqs }: Props) => (
  <div className="contextual-help-section">
    <div className="header">
      <Icon icon={IconKind.Chat} />
      <span className="title">{m.cmp_contextual_help_faq()}</span>
    </div>
    <div className="faq-card">
      {faqs.map((faq, i) => (
        <FaqItem key={i} question={faq.question} answer={faq.answer} />
      ))}
    </div>
  </div>
);
