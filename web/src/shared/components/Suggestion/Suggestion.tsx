import type { HTMLProps, ReactNode } from 'react';
import './style.scss';
import clsx from 'clsx';
import { AppText } from '../../defguard-ui/components/AppText/AppText';
import { Icon } from '../../defguard-ui/components/Icon';
import { TextStyle, ThemeVariable } from '../../defguard-ui/types';

type Props = Omit<HTMLProps<HTMLDivElement>, 'content' | 'title'> & {
  title: string;
  content: ReactNode;
};

export const Suggestion = ({ title, content, className, ...props }: Props) => {
  return (
    <div className={clsx('suggestion', className)} {...props}>
      <div className="suggestion-header">
        <Icon icon="light-bulb" staticColor={ThemeVariable.FgMuted} size={20} />
        <AppText font={TextStyle.TBodyPrimary600} color={ThemeVariable.FgDefault}>
          {title}
        </AppText>
      </div>
      <div className="suggestion-panel">{content}</div>
    </div>
  );
};
