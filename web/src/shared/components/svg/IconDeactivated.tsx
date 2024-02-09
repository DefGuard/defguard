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
      <clipPath id="icon-deactivated_svg__a">
        <path
          d="M0 0h22v22H0z"
          style={{
            fill: '#899ca8',
            opacity: 0,
          }}
        />
      </clipPath>
      <style>{'.icon-deactivated_svg__d{fill:#899ca8}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#icon-deactivated_svg__a)',
      }}
      transform="rotate(90 11 11)"
    >
      <g
        style={{
          fill: 'none',
          stroke: '#899ca8',
          strokeWidth: 2,
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
        className="icon-deactivated_svg__d"
        rx={1}
        transform="rotate(45 -3.07 10.83)"
      />
    </g>
  </svg>
);
export default SvgIconDeactivated;
