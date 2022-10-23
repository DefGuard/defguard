import './style.scss';

import { motion } from 'framer-motion';
import React from 'react';
import { Column, useTable } from 'react-table';

import { Webhook } from '../../../shared/types';
import { tableRowVariants } from '../../../shared/variants';
import WebhookEditButton from './WebhookEditButton';

interface Props {
  columns: Column<Webhook>[];
  data: Webhook[];
}

const WebhooksTable: React.FC<Props> = ({ data, columns }) => {
  const { getTableProps, getTableBodyProps, headerGroups, rows, prepareRow } =
    useTable({ columns, data });

  return (
    <div className="table-container">
      <table {...getTableProps()} className="webhooks-table">
        <thead>
          {headerGroups.map((headerGroup) => (
            // eslint-disable-next-line react/jsx-key
            <tr {...headerGroup.getHeaderGroupProps()}>
              {headerGroup.headers.map((column) => (
                // eslint-disable-next-line react/jsx-key
                <th {...column.getHeaderProps()}>{column.render('Header')}</th>
              ))}
              <th>
                <span>Actions</span>
              </th>
            </tr>
          ))}
        </thead>
        <tbody {...getTableBodyProps()}>
          {rows.map((row, index) => {
            prepareRow(row);
            return (
              // eslint-disable-next-line react/jsx-key
              <motion.tr
                custom={index}
                variants={tableRowVariants}
                initial={false}
                animate="idle"
                whileHover="hover"
                {...row.getRowProps()}
              >
                {row.cells.map((cell) => (
                  // eslint-disable-next-line react/jsx-key
                  <td {...cell.getCellProps()}>{cell.render('Cell')}</td>
                ))}
                <WebhookEditButton
                  id={row.original.id}
                  enabled={row.original.enabled}
                  webhook={row.original}
                />
              </motion.tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
};

export default WebhooksTable;
