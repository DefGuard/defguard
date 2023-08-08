import type { SVGProps } from 'react';
const SvgIconUserListHover = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="a">
        <path
          d="M0 0h22v22H0z"
          style={{
            opacity: 0,
            fill: '#899ca8',
          }}
        />
      </clipPath>
      <style>{'.c{fill:#899ca8}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#a)',
      }}
      transform="translate(-312 -227)"
    >
      <rect width={14} height={2} className="c" rx={1} transform="translate(316 233)" />
      <rect width={10} height={2} className="c" rx={1} transform="translate(316 237)" />
      <rect width={10} height={2} className="c" rx={1} transform="translate(316 241)" />
    </g>
  </svg>
);
export default SvgIconUserListHover;
