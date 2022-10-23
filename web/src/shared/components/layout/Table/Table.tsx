import './style.scss';

import { motion, Variant, Variants } from 'framer-motion';
import { ComponentPropsWithoutRef, useCallback, useMemo } from 'react';
import { Column, Row, useSortBy, useTable } from 'react-table';
import AutoSizer from 'react-virtualized-auto-sizer';
import { FixedSizeList, ListChildComponentProps } from 'react-window';

import { TableSortIcon } from './TableSortIcon';

interface Props<T extends object> extends ComponentPropsWithoutRef<'div'> {
  columns: Column<T>[];
  data: T[];
  rowHeight?: number;
}

// NOTE: It' not finished component.
export const Table = <T extends object>({
  columns,
  data,
  rowHeight = 60,
  className,
  ...rest
}: Props<T>) => {
  const { getTableProps, getTableBodyProps, rows, headers, prepareRow } =
    useTable({ columns, data }, useSortBy);

  const RenderRows = useCallback(
    ({ index, style }: ListChildComponentProps<T>) => {
      const row = rows[index];
      prepareRow(row);
      return <RowRender row={row} style={style} />;
    },
    [prepareRow, rows]
  );

  const getContainerClassName = useMemo(() => {
    const res = ['table-container'];
    if (className) {
      res.push(className);
    }
    return res.join(' ');
  }, [className]);

  return (
    <div {...getTableProps()} {...rest} className={getContainerClassName}>
      <div className="headers">
        {headers.map((column) => (
          // eslint-disable-next-line react/jsx-key
          <div
            {...column.getHeaderProps(column.getSortByToggleProps())}
            className="header"
          >
            <span>{column.render('Header')}</span>
            <SortIconRender
              sorted={column.isSorted}
              isSortedDesc={column.isSortedDesc}
            />
          </div>
        ))}
      </div>
      <div {...getTableBodyProps()} className="content">
        <AutoSizer>
          {({ height, width }) => (
            <FixedSizeList
              height={height}
              width={width}
              itemCount={rows.length}
              itemSize={rowHeight}
            >
              {RenderRows}
            </FixedSizeList>
          )}
        </AutoSizer>
      </div>
    </div>
  );
};

interface SortIconProps {
  sorted?: boolean;
  isSortedDesc?: boolean;
}

const SortIconRender: React.FC<SortIconProps> = ({ isSortedDesc, sorted }) => {
  const getAnimate = useMemo(() => {
    const variant: Variant = {
      rotate: 0,
    };
    if (!isSortedDesc) {
      variant.rotate = 180;
    }
    return variant;
  }, [isSortedDesc]);

  if (!sorted) {
    return null;
  }

  return <TableSortIcon animate={getAnimate} />;
};

interface RowRenderProps<T extends object> {
  row: Row<T>;
  style?: React.CSSProperties;
}

const RowRender = <T extends object>({ row, style }: RowRenderProps<T>) => {
  return (
    <motion.div
      {...row.getRowProps()}
      className="row"
      style={style}
      variants={tableRowVariants}
      whileHover="hover"
      initial={false}
      animate="idle"
    >
      {row.cells.map((cell) => (
        // eslint-disable-next-line react/jsx-key
        <div {...cell.getCellProps()} className="cell">
          {cell.render('Cell')}
        </div>
      ))}
    </motion.div>
  );
};

const tableRowVariants: Variants = {
  idle: () => ({
    boxShadow: '5px 10px 20px rgba(0, 0, 0, 0)',
  }),
  hover: {
    boxShadow: '5px 10px 20px rgba(0, 0, 0, 0.1)',
  },
};
