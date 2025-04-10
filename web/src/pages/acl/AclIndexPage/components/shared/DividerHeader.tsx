import { PropsWithChildren } from 'react';

type DividerHeaderProps = {
  text: string;
} & PropsWithChildren;

export const DividerHeader = ({ text, children }: DividerHeaderProps) => {
  return (
    <div className="divider-header spacer">
      <div className="inner">
        <p className="header">{text}</p>
        {children}
      </div>
    </div>
  );
};
