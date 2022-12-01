import './style.scss';

import { motion, Variants } from 'framer-motion';
import { Column, useTable } from 'react-table';

import SvgIconUserListHover from '../../../../shared/components/svg/IconUserListHover';
import { OpenidClient } from '../../../../shared/types';
import OpenidClientEditButton from './OpenidClientEditButton';

interface Props {
  columns: Column<OpenidClient>[];
  data: OpenidClient[];
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
    opacity: 0,
  },
  hover: {
    opacity: 1,
  },
};

const OpenidClientsListTable = ({ data, columns, ...rest }: Props) => {
  const { getTableProps, getTableBodyProps, headerGroups, rows, prepareRow } =
    useTable({ columns, data });
  return (
    <table {...getTableProps()} className="clients-list-table" {...rest}>
      <thead>
        {headerGroups.map((headerGroup) => (
          // eslint-disable-next-line react/jsx-key
          <tr {...headerGroup.getHeaderGroupProps()}>
            {headerGroup.headers.map((column) => (
              // eslint-disable-next-line react/jsx-key
              <th {...column.getHeaderProps()}>{column.render('Header')}</th>
            ))}
            <th className="edit">Actions</th>
          </tr>
        ))}
      </thead>
      <motion.tbody
        {...getTableBodyProps()}
        variants={tableBodyVariants}
        initial="hidden"
        animate="idle"
        layout
      >
        {rows.map((row) => {
          prepareRow(row);
          return (
            // eslint-disable-next-line react/jsx-key
            <motion.tr
              {...row.getRowProps()}
              variants={tableRowVariants}
              whileHover="hover"
              initial={false}
              animate="idle"
            >
              <motion.td className="row-icon" variants={rowIconVariants}>
                <SvgIconUserListHover />
              </motion.td>
              {row.cells.map((cell) => (
                // eslint-disable-next-line react/jsx-key
                <td {...cell.getCellProps()}>{cell.render('Cell')}</td>
              ))}
              <td className="row-edit">
                <OpenidClientEditButton client={row.original} />
              </td>
            </motion.tr>
          );
        })}
      </motion.tbody>
    </table>
  );
};

export default OpenidClientsListTable;
