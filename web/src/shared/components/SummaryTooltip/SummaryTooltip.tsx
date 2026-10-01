import './style.scss';

import type { Placement } from '@floating-ui/react';
import clsx from 'clsx';
import { Fragment, type PropsWithChildren, type ReactNode } from 'react';
import { Divider } from '../../defguard-ui/components/Divider/Divider';
import { IconKind } from '../../defguard-ui/components/Icon';
import { Icon } from '../../defguard-ui/components/Icon/Icon';
import { TooltipContent } from '../../defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../defguard-ui/providers/tooltip/TooltipTrigger';
import { ThemeVariable } from '../../defguard-ui/types';
import type { SummarySection } from './type';

type SummaryTooltipLine =
  | string
  | {
      text: string;
      warning?: boolean;
    };

type SummaryTooltipSection = Omit<SummarySection, 'lines'> & {
  lines: SummaryTooltipLine[];
};

type Props = {
  sections: SummaryTooltipSection[];
  className?: string;
  placement?: Placement;
  footer?: ReactNode;
};

export const SummaryTooltip = ({
  sections,
  className,
  children,
  placement = 'right-start',
  footer,
}: Props & PropsWithChildren) => {
  if (sections.length === 0) return children;

  return (
    <TooltipProvider placement={placement}>
      <TooltipTrigger>{children}</TooltipTrigger>
      <TooltipContent className={clsx('summary-tooltip', className)}>
        {sections.map((section, index) => (
          <Fragment key={section.label}>
            {index > 0 && <Divider />}
            <div className="summary-item">
              <p className="label">{section.label}</p>
              <div className="content">
                {section.lines.map((line) => {
                  const summaryLine: SummaryTooltipLine =
                    typeof line === 'string' ? { text: line } : line;
                  return (
                    <p key={`${section.label}-${summaryLine.text}`}>
                      {summaryLine.text}
                      {summaryLine.warning && (
                        <Icon
                          icon={IconKind.WarningFilled}
                          size={16}
                          staticColor={ThemeVariable.FgCritical}
                        />
                      )}
                    </p>
                  );
                })}
              </div>
            </div>
          </Fragment>
        ))}
        {footer && (
          <>
            <Divider />
            <div className="footer">{footer}</div>
          </>
        )}
      </TooltipContent>
    </TooltipProvider>
  );
};
