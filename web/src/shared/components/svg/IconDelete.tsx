import type { SVGProps } from 'react';
const SvgIconDelete = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="icon-delete_svg__a">
        <path
          d="M0 0h22v22H0z"
          style={{
            opacity: 0,
            fill: '#cb3f3f',
          }}
        />
      </clipPath>
      <style>{'.icon-delete_svg__d{fill:#cb3f3f}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#icon-delete_svg__a)',
      }}
      transform="rotate(90 11 11)"
    >
      <g
        style={{
          stroke: '#cb3f3f',
          strokeWidth: 2,
          fill: 'none',
        }}
        transform="translate(3 3)"
      >
        <circle
          cx={8}
          cy={8}
          r={8}
          style={{
            stroke: 'none',
          }}
        />
        <circle
          cx={8}
          cy={8}
          r={7}
          style={{
            fill: 'none',
          }}
        />
      </g>
      <rect
        width={8}
        height={2}
        className="icon-delete_svg__d"
        rx={1}
        transform="rotate(45 -4.57 14.45)"
      />
      <rect
        width={8}
        height={2}
        className="icon-delete_svg__d"
        rx={1}
        transform="rotate(135 5.43 7.45)"
      />
    </g>
  </svg>
);
export default SvgIconDelete;
