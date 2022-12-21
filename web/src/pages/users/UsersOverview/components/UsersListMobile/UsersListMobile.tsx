import { motion } from 'framer-motion';
import { clone, toInteger } from 'lodash-es';
import React, {
  ComponentPropsWithoutRef,
  createRef,
  forwardRef,
  useCallback,
  useState,
} from 'react';
import AutoSizer from 'react-virtualized-auto-sizer';
import { ListChildComponentProps, VariableSizeList } from 'react-window';

import NoData from '../../../../../shared/components/layout/NoData/NoData';
import { User } from '../../../../../shared/types';
import { tableRowVariants } from '../../../../../shared/variants';
import UserListItem from './UserListItem';

interface Props {
  users: User[];
}

const UsersListMobile: React.FC<Props> = ({ users }) => {
  const [expandedRows, setExpandedRows] = useState<Array<boolean>>(
    Array(users.length > 0 ? users.length : 0).fill(false)
  );
  const ref = createRef<VariableSizeList<User>>();

  const getItemSize = useCallback(
    (index: number) => {
      const isExpanded = expandedRows[index];
      if (isExpanded) {
        return EXPANDED_ROW_HEIGHT + GAP_SIZE;
      }
      return ROW_HEIGHT + GAP_SIZE;
    },
    [expandedRows]
  );

  const changeExpand = useCallback(
    (index: number) => {
      const dump = clone(expandedRows);
      dump[index] = !dump[index];
      setExpandedRows(dump);
      ref.current?.resetAfterIndex(index);
    },
    [expandedRows, ref]
  );

  const renderRow = useCallback(
    ({ index, style }: ListChildComponentProps<User>) => {
      const user = users[index];

      const isExpanded = expandedRows[index];
      return (
        <motion.div
          key={user.username}
          custom={index}
          variants={tableRowVariants}
          initial="hidden"
          animate="idle"
          style={{
            ...style,
            height: toInteger(style.height ?? ROW_HEIGHT) - GAP_SIZE,
            top: toInteger(style.top ?? 0) + GAP_SIZE,
          }}
          className="row"
        >
          <UserListItem
            user={user}
            expanded={isExpanded}
            onChangeExpand={() => changeExpand(index)}
          />
        </motion.div>
      );
    },
    [changeExpand, expandedRows, users]
  );

  if (users.length === 0) return <NoData customMessage="No users found" />;

  return (
    <div className="users-list-mobile">
      <AutoSizer>
        {({ height, width }) => (
          <VariableSizeList
            width={width}
            height={height}
            innerElementType={innerElementType}
            itemCount={users.length}
            itemSize={(index) => getItemSize(index)}
            ref={ref}
          >
            {renderRow}
          </VariableSizeList>
        )}
      </AutoSizer>
    </div>
  );
};

export default UsersListMobile;

const innerElementType = forwardRef<
  HTMLDivElement,
  ComponentPropsWithoutRef<'div'>
>(({ style, ...rest }, ref) => (
  <div
    ref={ref}
    style={{
      ...style,
      height: `${
        parseFloat(String(style?.height ?? 0)) + SCROLL_BOX_PADDING * 2
      }px`,
    }}
    {...rest}
  />
));

const GAP_SIZE = 10;
const ROW_HEIGHT = 60;
const SCROLL_BOX_PADDING = 5;
const EXPANDED_ROW_HEIGHT = 133;
