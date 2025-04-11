import './style.scss';

import clsx from 'clsx';
import { useCallback, useId, useMemo, useState } from 'react';

import { FieldError } from '../../../../../shared/defguard-ui/components/Layout/FieldError/FieldError';
import { FloatingMenu } from '../../../../../shared/defguard-ui/components/Layout/FloatingMenu/FloatingMenu';
import { FloatingMenuProvider } from '../../../../../shared/defguard-ui/components/Layout/FloatingMenu/FloatingMenuProvider';
import { FloatingMenuTrigger } from '../../../../../shared/defguard-ui/components/Layout/FloatingMenu/FloatingMenuTrigger';
import { Label } from '../../../../../shared/defguard-ui/components/Layout/Label/Label';
import { isPresent } from '../../../../../shared/defguard-ui/utils/isPresent';
import { DialogSelectModal } from './DialogSelectModal/DialogSelectModal';
import { DialogSelectProps } from './types';

export const DialogSelect = <T extends object, I extends number | string>({
  options,
  selected,
  identKey,
  label,
  onChange,
  renderTagContent,
  renderDialogListItem,
  searchFn,
  searchKeys,
  errorMessage,
  disabled = false,
}: DialogSelectProps<T, I>) => {
  const [modalOpen, setModalOpen] = useState(false);
  const getIdent = useCallback((val: T): I => val[identKey] as I, [identKey]);

  const selectedOptions = useMemo(
    () => options.filter((o) => selected.includes(getIdent(o))),
    [getIdent, options, selected],
  );

  const error = !disabled ? errorMessage : undefined;

  const getLabel = renderDialogListItem ? renderDialogListItem : renderTagContent;

  return (
    <>
      <div className="dialog-select spacer">
        <div className="inner">
          {label !== undefined && <Label>{label}</Label>}
          <div
            className={clsx('dialog-select-field', {
              disabled,
              invalid: isPresent(error),
            })}
          >
            <FloatingMenuProvider placement="top">
              <FloatingMenuTrigger asChild>
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
              </FloatingMenuTrigger>
              <FloatingMenu className="dialog-select-track-floating-menu">
                <ul>
                  {selectedOptions.map((o) => {
                    const id = getIdent(o);
                    return <li key={id}>{getLabel(o)}</li>;
                  })}
                </ul>
              </FloatingMenu>
            </FloatingMenuProvider>
            <button
              disabled={disabled}
              className="open-button"
              onClick={() => {
                setModalOpen(true);
              }}
              type="button"
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
          <FieldError errorMessage={error} />
        </div>
      </div>
      <DialogSelectModal
        searchFn={searchFn}
        searchKeys={searchKeys}
        open={modalOpen}
        setOpen={setModalOpen}
        options={options}
        getIdent={getIdent}
        initiallySelected={selected}
        getLabel={getLabel}
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
