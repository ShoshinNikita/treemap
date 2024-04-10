import type { Meta, StoryObj } from '@storybook/react';
import { MapBase } from "./MapBase";
import { DEFAULT_MAP_CENTER } from "@/utils/config";

const meta = {
  title: 'Maps/MapBase',
  component: MapBase,
  parameters: {
    layout: 'fullscreen', // centered
  },
} satisfies Meta<typeof MapBase>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Desktop: Story = {
  args: {
    center: DEFAULT_MAP_CENTER,
  },
  parameters: {
    viewport: {
      defaultViewport: "responsive",
    },
  },
};

export const Phone: Story = {
  args: {
    center: DEFAULT_MAP_CENTER,
  },
  parameters: {
    viewport: {
      defaultViewport: "mobile1",
    },
  },
};
