import './style.scss';

import useResizeObserver from '@react-hook/resize-observer';
import clsx from 'clsx';
import { useCallback, useRef, useState } from 'react';

import { FloatingMenu } from '../../../../../../shared/defguard-ui/components/Layout/FloatingMenu/FloatingMenu';
import { FloatingMenuProvider } from '../../../../../../shared/defguard-ui/components/Layout/FloatingMenu/FloatingMenuProvider';
import { FloatingMenuTrigger } from '../../../../../../shared/defguard-ui/components/Layout/FloatingMenu/FloatingMenuTrigger';
import { Tag } from '../../../../../../shared/defguard-ui/components/Layout/Tag/Tag';
import { isPresent } from '../../../../../../shared/defguard-ui/utils/isPresent';
import { ListTagDisplay } from '../types';

type RenderTagsProps = {
  data: ListTagDisplay[];
  placeholder?: string;
};

export const RenderTagDisplay = ({ data, placeholder }: RenderTagsProps) => {
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
          className={clsx('tags-display', {
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
    <ul className="acl-floating-tags-display">
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
