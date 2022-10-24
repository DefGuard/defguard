import './style.scss';

import { motion, Variants } from 'framer-motion';
import { toInteger } from 'lodash-es';
import React, {
  ComponentPropsWithoutRef,
  forwardRef,
  useCallback,
} from 'react';
import { Column, useTable } from 'react-table';
import AutoSizer from 'react-virtualized-auto-sizer';
import { FixedSizeList, ListChildComponentProps } from 'react-window';

import { User } from '../../../../shared/types';
import UserEditButton from './UserEditButton';

interface Props {
  columns: Column<User>[];
  data: User[];
  navigateToUser: (user: User) => void;
}

const tableBodyVariants: Variants = {
  hidden: {
    opacity: 0,
  },
  idle: {
    opacity: 1,
  },
};

const tableRowVariants: Variants = {
  idle: () => ({
    boxShadow: '5px 10px 20px rgba(0, 0, 0, 0)',
  }),
  hover: {
    boxShadow: '5px 10px 20px rgba(0, 0, 0, 0.1)',
  },
};

const rowIconVariants: Variants = {
  idle: {
    opacity: 0.4,
  },
  hover: {
    opacity: 1,
  },
};

const UsersListTable: React.FC<ComponentPropsWithoutRef<'table'> & Props> = ({
  data,
  columns,
  navigateToUser,
  ...rest
}) => {
  const { getTableProps, getTableBodyProps, headerGroups, rows, prepareRow } =
    useTable({ columns, data });

  const renderRows = useCallback(
    ({ index, style }: ListChildComponentProps<User>) => {
      const row = rows[index];
      prepareRow(row);
      return (
        <motion.div
          {...row.getRowProps()}
          variants={tableRowVariants}
          whileHover="hover"
          initial={false}
          animate="idle"
          style={{
            ...style,
            height: toInteger(style.height ?? ROW_HEIGHT) - GAP_SIZE,
            top: toInteger(style.top ?? 0) + GAP_SIZE,
          }}
          className="row"
        >
          <motion.div className="cell row-icon" variants={rowIconVariants}>
            {/* <SvgIconUserListHover /> */}
          </motion.div>
          {row.cells.map((cell) => (
            // eslint-disable-next-line react/jsx-key
            <div
              className="cell pointer"
              {...cell.getCellProps()}
              onClick={() => navigateToUser(row.original)}
            >
              {cell.render('Cell')}
            </div>
          ))}
          <div className="cell row-edit">
            <UserEditButton user={row.original} />
          </div>
        </motion.div>
      );
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [prepareRow, rows]
  );

  return (
    <div {...getTableProps()} className="users-list-wrapper" {...rest}>
      <div className="headers">
        {headerGroups.map((headerGroup) => (
          // eslint-disable-next-line react/jsx-key
          <div className="row" {...headerGroup.getHeaderGroupProps()}>
            {headerGroup.headers.map((column) => (
              // eslint-disable-next-line react/jsx-key
              <div className="header" {...column.getHeaderProps()}>
                {column.render('Header')}
              </div>
            ))}
            <div className="header">Actions</div>
          </div>
        ))}
      </div>
      <motion.div
        {...getTableBodyProps()}
        className="body"
        variants={tableBodyVariants}
        initial="hidden"
        animate="idle"
        layout
      >
        <AutoSizer>
          {({ height, width }) => (
            <FixedSizeList
              height={height}
              width={width}
              itemCount={rows.length}
              itemSize={ROW_HEIGHT + GAP_SIZE}
              innerElementType={innerElementType}
            >
              {renderRows}
            </FixedSizeList>
          )}
        </AutoSizer>
      </motion.div>
    </div>
  );
};

export default UsersListTable;

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
