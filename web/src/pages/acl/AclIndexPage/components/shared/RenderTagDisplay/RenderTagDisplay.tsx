import './style.scss';

import { FloatingMenu } from '../../../../../../shared/defguard-ui/components/Layout/FloatingMenu/FloatingMenu';
import { FloatingMenuProvider } from '../../../../../../shared/defguard-ui/components/Layout/FloatingMenu/FloatingMenuProvider';
import { FloatingMenuTrigger } from '../../../../../../shared/defguard-ui/components/Layout/FloatingMenu/FloatingMenuTrigger';
import { Tag } from '../../../../../../shared/defguard-ui/components/Layout/Tag/Tag';
import { ListTagDisplay } from '../types';

type RenderTagsProps = {
  data: ListTagDisplay[];
};

export const RenderTagDisplay = ({ data }: RenderTagsProps) => {
  return (
    <FloatingMenuProvider placement="right">
      <FloatingMenuTrigger asChild>
        <div className="tags-display">
          <TagContent data={data} />
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
