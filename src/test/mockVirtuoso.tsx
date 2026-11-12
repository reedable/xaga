import type { ComponentType, Key, ReactNode } from "react";
import type { ItemProps } from "react-virtuoso";

// jsdom has no layout. Exercise row interactions here; browser tests cover virtualization.
export const Virtuoso = ({
  data,
  itemContent,
  computeItemKey,
  components,
  context,
}: {
  data: ReactNode[];
  itemContent: (index: number, item: ReactNode) => ReactNode;
  computeItemKey: (index: number, item: ReactNode) => Key;
  components: {
    Item: ComponentType<
      ItemProps<ReactNode> & { context?: { itemCount: number } }
    >;
  };
  context: { itemCount: number };
}) => {
  const Item = components.Item;
  return (
    <div data-virtuoso-scroller>
      {data.map((item, index) => (
        <Item
          key={computeItemKey(index, item)}
          item={item}
          context={context}
          data-index={index}
          data-item-index={index}
          data-known-size={0}
        >
          {itemContent(index, item)}
        </Item>
      ))}
    </div>
  );
};
