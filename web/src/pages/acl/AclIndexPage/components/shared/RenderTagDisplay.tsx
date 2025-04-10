import { Tag } from '../../../../../shared/defguard-ui/components/Layout/Tag/Tag';
import { ListTagDisplay } from './types';

type RenderTagsProps = {
  data: ListTagDisplay[];
};

export const RenderTagDisplay = ({ data }: RenderTagsProps) => {
  return (
    <div className="tags-display">
      <div className="track">
        {data.map((d) => {
          if (d.displayAsTag) {
            return <Tag key={d.key} text={d.label} />;
          }
          return <span key={d.key}>{d.label}</span>;
        })}
      </div>
    </div>
  );
};
