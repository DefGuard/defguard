import './style.scss';

import clsx from 'clsx';
import { PropsWithChildren, useState } from 'react';

import { ArrowSingle } from '../../../defguard-ui/components/icons/ArrowSingle/ArrowSingle';
import { ArrowSingleDirection } from '../../../defguard-ui/components/icons/ArrowSingle/types';

type Props = {
  text: string;
  initOpen?: boolean;
  textAs?: 'h1' | 'h2' | 'h3' | 'h4' | 'h5' | 'h6' | 'p';
} & PropsWithChildren;

export const ExpandableSection = ({
  children,
  text,
  textAs: Tag = 'p',
  initOpen = true,
}: Props) => {
  const [expanded, setExpanded] = useState(initOpen);

  return (
    <div className="expandable-section spacer">
      <div
        className="track"
        onClick={() => {
          setExpanded((s) => !s);
        }}
      >
        <Tag>{text}</Tag>
        <ArrowSingle
          direction={expanded ? ArrowSingleDirection.DOWN : ArrowSingleDirection.RIGHT}
        />
      </div>
      <div
        className={clsx('expandable', {
          open: expanded,
        })}
      >
        <div>{children}</div>
      </div>
    </div>
  );
};
