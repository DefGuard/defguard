import type { SVGProps } from 'react';
const SvgIconUserListExpanded = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="a">
        <path d="M0 0h22v22H0z" className="a" />
      </clipPath>
      <clipPath id="b">
        <path
          d="M0 0h22v22H0z"
          style={{
            opacity: 0,
            fill: '#0c8ce0',
          }}
        />
      </clipPath>
      <style>{'.a{fill:#0c8ce0}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#a)',
      }}
      transform="rotate(90 11 11)"
    >
      <g transform="rotate(-90 53.5 280.5)">
        <rect width={10} height={2} className="a" rx={1} transform="translate(320 233)" />
        <rect width={6} height={2} className="a" rx={1} transform="translate(320 237)" />
        <rect width={2} height={2} className="a" rx={1} transform="translate(316 237)" />
        <rect width={2} height={2} className="a" rx={1} transform="translate(316 241)" />
        <rect width={2} height={2} className="a" rx={1} transform="translate(316 233)" />
        <rect width={6} height={2} className="a" rx={1} transform="translate(320 241)" />
      </g>
      <g
        style={{
          clipPath: 'url(#b)',
        }}
        transform="rotate(180 13 2)"
      >
        <rect
          width={8}
          height={2}
          className="a"
          rx={1}
          transform="rotate(45 -6.9 16.07)"
        />
        <rect
          width={8}
          height={2}
          className="a"
          rx={1}
          transform="rotate(135 6.172 6.314)"
        />
      </g>
    </g>
  </svg>
);
export default SvgIconUserListExpanded;
