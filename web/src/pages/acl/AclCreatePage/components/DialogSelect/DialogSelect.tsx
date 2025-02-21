import './style.scss';

import clsx from 'clsx';
import { useCallback, useId, useMemo, useState } from 'react';

import { Label } from '../../../../../shared/defguard-ui/components/Layout/Label/Label';
import { DialogSelectModal } from './DialogSelectModal/DialogSelectModal';
import { DialogSelectProps } from './types';

export const DialogSelect = <T extends object, I extends number | string>({
  options,
  selected,
  identKey,
  label,
  onChange,
  renderTagContent,
}: DialogSelectProps<T, I>) => {
  const [modalOpen, setModalOpen] = useState(false);
  const getIdent = useCallback((val: T): I => val[identKey] as I, [identKey]);

  const selectedOptions = useMemo(
    () => options.filter((o) => selected.includes(getIdent(o))),
    [getIdent, options, selected],
  );

  return (
    <>
      <div className="dialog-select-spacer">
        {label !== undefined && <Label>{label}</Label>}
        <div className={clsx('dialog-select')}>
          <div className={clsx('track')}>
            <div className="options">
              {renderTagContent !== undefined &&
                selectedOptions.map((o) => {
                  const id = getIdent(o);
                  return (
                    <div className="dialog-select-tag" key={id}>
                      {renderTagContent(o)}
                    </div>
                  );
                })}
            </div>
            <TrackGradient />
          </div>
          <button
            className="open-button"
            onClick={() => {
              setModalOpen(true);
            }}
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="22"
              height="22"
              viewBox="0 0 22 22"
              fill="none"
            >
              <path d="M5.5 11H16.5" strokeWidth="2" strokeLinecap="round" />
              <path d="M11 5.5L11 16.5" strokeWidth="2" strokeLinecap="round" />
            </svg>
          </button>
        </div>
      </div>
      <DialogSelectModal
        open={modalOpen}
        setOpen={setModalOpen}
        options={options}
        getIdent={getIdent}
        initiallySelected={selected}
        getLabel={renderTagContent}
        onChange={(vals) => {
          onChange?.(vals);
        }}
      />
    </>
  );
};

const TrackGradient = () => {
  const id = useId();
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={65}
      height={48}
      viewBox="0 0 65 48"
      fill="none"
    >
      <rect width={65} height={48} fill={`url(#${id})`} />
      <defs>
        <linearGradient
          id={id}
          x1={-4.13636}
          y1={48}
          x2={32.5}
          y2={48}
          gradientUnits="userSpaceOnUse"
        >
          <stop stopOpacity={0} style={{ stopColor: 'var(--surface-frame-bg)' }} />
          <stop
            offset={1}
            stopOpacity={0.9}
            style={{ stopColor: 'var(--surface-frame-bg)' }}
          />
        </linearGradient>
      </defs>
    </svg>
  );
};
