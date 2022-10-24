import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AnimatePresence, motion } from 'framer-motion';
import React, { useState } from 'react';
import ClickAwayListener from 'react-click-away-listener';
import { Column, useTable } from 'react-table';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import IconButton from '../../../shared/components/layout/IconButton/IconButton';
import OptionsPopover from '../../../shared/components/layout/OptionsPopover/OptionsPopover';
import SvgIconEditAlt from '../../../shared/components/svg/IconEditAlt';
import useApi from '../../../shared/hooks/useApi';
import { MutationKeys } from '../../../shared/mutations';
import { QueryKeys } from '../../../shared/queries';
import { Provisioner } from '../../../shared/types';
import { standardVariants, tableRowVariants } from '../../../shared/variants';

interface Props {
  columns: Column<Provisioner>[];
  data: Provisioner[];
}

const ProvisionersTable: React.FC<Props> = ({ data, columns }) => {
  const { getTableProps, getTableBodyProps, headerGroups, rows, prepareRow } =
    useTable({ columns, data });

  return (
    <div className="table-container">
      <table {...getTableProps()} className="provisioners-table">
        <thead>
          {headerGroups.map((headerGroup) => (
            // eslint-disable-next-line react/jsx-key
            <tr {...headerGroup.getHeaderGroupProps()}>
              {headerGroup.headers.map((column) => (
                // eslint-disable-next-line react/jsx-key
                <th {...column.getHeaderProps()}>{column.render('Header')}</th>
              ))}
              <th>
                <span>Edit</span>
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
                <EditTd id={row.original.id} />
              </motion.tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
};

export default ProvisionersTable;

type EditButtonProps = {
  id: string;
};

const EditTd: React.FC<EditButtonProps> = ({ id }) => {
  const [refElement, setRefElement] = useState<HTMLButtonElement | null>();
  const [optionsVisible, setOptionsVisible] = useState(false);
  const [rowOverlayVisible, setRowOverlayVisible] = useState(false);

  const {
    provisioning: { deleteWorker },
  } = useApi();

  const queryClient = useQueryClient();

  const { mutate, isLoading } = useMutation(deleteWorker, {
    mutationKey: [MutationKeys.DELETE_WORKER],
    onSuccess: () => {
      setRowOverlayVisible(false);
      queryClient.invalidateQueries([QueryKeys.FETCH_WORKERS]);
    },
  });

  return (
    <>
      <td className="edit" role="cell">
        <IconButton className="blank" ref={setRefElement}>
          <SvgIconEditAlt />
        </IconButton>
      </td>
      {refElement ? (
        <OptionsPopover
          referenceElement={refElement}
          isOpen={optionsVisible}
          setIsOpen={setOptionsVisible}
          items={[
            <button
              key="delete"
              className="warning"
              onClick={() => {
                setOptionsVisible(false);
                setRowOverlayVisible(true);
              }}
            >
              Delete
            </button>,
          ]}
        />
      ) : null}
      <AnimatePresence mode="wait">
        {rowOverlayVisible ? (
          <ClickAwayListener onClickAway={() => setRowOverlayVisible(false)}>
            <motion.div
              className="row-overlay delete"
              initial="hidden"
              animate="show"
              exit="hidden"
              variants={standardVariants}
            >
              <Button
                size={ButtonSize.SMALL}
                styleVariant={ButtonStyleVariant.STANDARD}
                onClick={() => setRowOverlayVisible(false)}
                disabled={isLoading}
                text="Cancel"
              />
              <Button
                styleVariant={ButtonStyleVariant.CONFIRM_WARNING}
                size={ButtonSize.SMALL}
                loading={isLoading}
                onClick={() => mutate(id)}
                text="Delete"
              />
            </motion.div>
          </ClickAwayListener>
        ) : null}
      </AnimatePresence>
    </>
  );
};
