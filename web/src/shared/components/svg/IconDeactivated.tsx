import type { SVGProps } from 'react';
const SvgIconDeactivated = (props: SVGProps<SVGSVGElement>) => (
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
      <style>{'.d{fill:#899ca8}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#a)',
      }}
      transform="rotate(90 11 11)"
    >
      <g
        style={{
          stroke: '#899ca8',
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
        width={14}
        height={2}
        className="d"
        rx={1}
        transform="rotate(45 -3.07 10.83)"
      />
    </g>
  </svg>
);
export default SvgIconDeactivated;
