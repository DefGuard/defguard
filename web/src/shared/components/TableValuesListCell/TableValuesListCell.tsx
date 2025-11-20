import './style.scss';
import clsx from 'clsx';
import { TableCell } from '../../defguard-ui/components/table/TableCell/TableCell';
import type { TableCellProps } from '../../defguard-ui/components/table/TableCell/types';
import { openModal } from '../../hooks/modalControls/modalsSubjects';
import { ModalName } from '../../hooks/modalControls/modalTypes';

interface Props extends TableCellProps {
  values: string[];
}

export const TableValuesListCell = ({ values, className, ...cellProps }: Props) => {
  return (
    <TableCell
      className={clsx(className, 'values-list')}
      {...cellProps}
      onClick={(e) => {
        e.preventDefault();
        e.stopPropagation();
        openModal(ModalName.DisplayList, {
          data: values,
        });
      }}
    >
      <span>{values.join(', ')}</span>
    </TableCell>
  );
};
