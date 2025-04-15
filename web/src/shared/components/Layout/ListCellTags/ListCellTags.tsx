import './style.scss';

import useResizeObserver from '@react-hook/resize-observer';
import clsx from 'clsx';
import { useCallback, useRef, useState } from 'react';

import { ListCellTag } from '../../../../pages/acl/AclIndexPage/components/shared/types';
import { FloatingMenu } from '../../../defguard-ui/components/Layout/FloatingMenu/FloatingMenu';
import { FloatingMenuProvider } from '../../../defguard-ui/components/Layout/FloatingMenu/FloatingMenuProvider';
import { FloatingMenuTrigger } from '../../../defguard-ui/components/Layout/FloatingMenu/FloatingMenuTrigger';
import { Tag } from '../../../defguard-ui/components/Layout/Tag/Tag';
import { isPresent } from '../../../defguard-ui/utils/isPresent';

type RenderTagsProps = {
  data: ListCellTag[];
  placeholder?: string;
};

export const ListCellTags = ({ data, placeholder }: RenderTagsProps) => {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [overflows, setOverflows] = useState(false);

  const handleResize = useCallback(() => {
    if (containerRef.current) {
      setOverflows(containerRef.current.scrollWidth > containerRef.current.clientWidth);
    }
  }, []);

  useResizeObserver(containerRef, handleResize);
  return (
    <FloatingMenuProvider placement="right" disabled={data.length === 0}>
      <FloatingMenuTrigger asChild>
        <div
          className={clsx('list-cell-tags', {
            empty: data.length === 0,
            overflows,
          })}
          ref={containerRef}
        >
          <TagContent data={data} />
          {data.length === 0 && isPresent(placeholder) && (
            <p className="placeholder">{placeholder}</p>
          )}
        </div>
      </FloatingMenuTrigger>
      <FloatingMenu>
        <FloatingContent data={data} />
      </FloatingMenu>
    </FloatingMenuProvider>
  );
};

const FloatingContent = ({ data }: RenderTagsProps) => {
  return (
    <ul className="list-cell-tags-floating">
      {data.map((d) => (
        <li key={d.key}>{d.label}</li>
      ))}
    </ul>
  );
};

const TagContent = ({ data }: RenderTagsProps) => {
  return (
    <div className="track">
      {data.map((d) => {
        if (d.displayAsTag) {
          return <Tag key={d.key} text={d.label} />;
        }
        return <span key={d.key}>{d.label}</span>;
      })}
    </div>
  );
};
