import classNames from 'classnames';
import { getTextWidth } from 'get-text-width';
import { useEffect, useState } from 'react';

import { TextContainer } from '../../../../../../shared/defguard-ui/components/Layout/TextContainer/TextContainer';
import { useUserGroupsListModal } from '../modals/UserGroupsListModal/useUserGroupsListModal';

type Props = {
  groups: string[];
};

const targetWidth = 360;

const itemGap = 10;

const border = 2;

// summary of both sides
const itemHorizontalPadding = 20;

const dotsBox = 38;

// cropped by css
const maxTextWidth = 135;

const calcElementWidth = (textWidth: number) => {
  return textWidth + itemHorizontalPadding + border;
};

export const UsersListGroups = ({ groups }: Props) => {
  const [displayGroups, setDisplayGroups] = useState<string[]>([]);
  const [enabledModal, setEnabledModal] = useState(false);
  const openModal = useUserGroupsListModal((s) => s.open);

  useEffect(() => {
    const toDisplay = [];
    let totalWidth = 0;
    let enable = false;
    for (const g of groups.sort((a, b) => a.localeCompare(b))) {
      let textWidth = 0;
      const estimatedTextWidth = getTextWidth(g);
      // if any group name will get cropped enable modal regardless if rest will fit
      if (estimatedTextWidth > maxTextWidth) {
        textWidth = maxTextWidth;
        enable = true;
        totalWidth += dotsBox;
      } else {
        textWidth = estimatedTextWidth;
      }
      const estimatedElWidth = calcElementWidth(textWidth);
      // add gap
      if (toDisplay.length > 0) {
        totalWidth += itemGap;
      }
      totalWidth += estimatedElWidth;
      // check if should display
      if (totalWidth <= targetWidth) {
        toDisplay.push(g);
      } else {
        // check if last element should be popped
        if (totalWidth - estimatedElWidth + (enable ? 0 : dotsBox) > targetWidth) {
          setDisplayGroups(toDisplay.slice(0, -1));
        } else {
          setDisplayGroups(toDisplay);
        }
        enable = true;
        break;
      }
    }
    if (enable) {
      setEnabledModal(true);
    } else {
      setEnabledModal(false);
      setDisplayGroups(toDisplay);
    }
  }, [groups]);

  return (
    <div
      className={classNames('groups-cell', {
        clickable: enabledModal,
      })}
      onClick={() => {
        if (enabledModal) {
          openModal(groups);
        }
      }}
    >
      {displayGroups.map((g, index) => (
        <div className="group" key={`${g}-${index}`}>
          <TextContainer text={g} />
        </div>
      ))}
      {enabledModal && (
        <div className="group">
          <span>...</span>
        </div>
      )}
    </div>
  );
};
