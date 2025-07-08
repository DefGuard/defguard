import './style.scss';

import { InteractionBox } from '../../../defguard-ui/components/Layout/InteractionBox/InteractionBox';
import SvgIconCopy from '../../../defguard-ui/components/svg/IconCopy';
import { isPresent } from '../../../defguard-ui/utils/isPresent';
import { ListCellText } from '../ListCellText/ListCellText';

type Props = {
  label?: string;
  value: string;
  onCopy: (value: string) => void;
};

export const CopyField = ({ onCopy, value, label }: Props) => {
  return (
    <div className="copy-field spacer">
      {isPresent(label) && label.length > 0 && <p className="label">{label}</p>}
      <div className="box">
        <div className="track">
          <ListCellText placement="bottom" text={value} />
          <div className="copy">
            <InteractionBox
              onClick={() => {
                onCopy(value);
              }}
            >
              <SvgIconCopy />
            </InteractionBox>
          </div>
        </div>
      </div>
    </div>
  );
};
