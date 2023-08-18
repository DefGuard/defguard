import type { SVGProps } from 'react';
const SvgIconActivityRemoved = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={16}
    height={16}
    viewBox="0 0 16 16"
    {...props}
  >
    <defs>
      <clipPath id="icon-activity-removed_svg__a">
        <path d="M0 0h16v16H0z" className="icon-activity-removed_svg__a" />
      </clipPath>
      <style>{'.icon-activity-removed_svg__a{fill:#cb3f3f}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#icon-activity-removed_svg__a)',
      }}
    >
      <g
        style={{
          fill: 'none',
        }}
      >
        <path
          d="M6 0a6 6 0 1 1-6 6 6 6 0 0 1 6-6Z"
          style={{
            stroke: 'none',
          }}
          transform="rotate(90 6 8)"
        />
        <path
          d="M6 2C3.794 2 2 3.794 2 6s1.794 4 4 4 4-1.794 4-4-1.794-4-4-4m0-2a6 6 0 1 1 0 12A6 6 0 0 1 6 0Z"
          style={{
            fill: '#cb3f3f',
            stroke: 'none',
          }}
          transform="rotate(90 6 8)"
        />
      </g>
      <rect
        width={10}
        height={2}
        className="icon-activity-removed_svg__a"
        rx={1}
        transform="rotate(135 5.05 5.122)"
      />
    </g>
  </svg>
);
export default SvgIconActivityRemoved;
