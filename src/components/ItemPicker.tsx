import React, { useState, useEffect, useCallback, useRef } from "react";
import { Modal } from "./Modal.js";
import type { Item } from "../../component-built/interfaces/local-module-module.d.ts";

interface ItemPickerProps {
  items: Item[];
  onSelect: (item: Item) => void;
  onClose: () => void;
  title?: string;
}

export const ItemPicker: React.FC<ItemPickerProps> = ({
  items,
  onSelect,
  onClose,
  title,
}) => {
  const [highlightedIndex, setHighlightedIndex] = useState(0);
  const highlightedItemRef = useRef<HTMLLIElement>(null);

  // Reset highlighted index when modal opens or items change
  useEffect(() => {
    setHighlightedIndex(0);
  }, [items]);

  const scrollToHighlighted = useCallback(() => {
    if (highlightedItemRef.current) {
      highlightedItemRef.current.scrollIntoView({
        behavior: "instant",
        block: "center",
      });
    }
  }, []);

  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      switch (event.key) {
        case "ArrowDown":
          event.preventDefault();
          setHighlightedIndex((prev) => {
            const newIndex = prev < items.length - 1 ? prev + 1 : prev;
            setTimeout(scrollToHighlighted, 0);
            return newIndex;
          });
          break;
        case "ArrowUp":
          event.preventDefault();
          setHighlightedIndex((prev) => {
            const newIndex = prev > 0 ? prev - 1 : prev;
            setTimeout(scrollToHighlighted, 0);
            return newIndex;
          });
          break;
        case "Enter":
          event.preventDefault();
          if (items[highlightedIndex]) {
            onSelect(items[highlightedIndex]);
          }
          break;
        case "Escape":
          event.preventDefault();
          onClose();
          break;
      }
    },
    [items, highlightedIndex, onSelect, onClose, scrollToHighlighted],
  );

  useEffect(() => {
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  const handleItemClick = (item: Item, index: number) => {
    setHighlightedIndex(index);
    onSelect(item);
  };

  return (
    <Modal onClose={onClose}>
      {title && (
        <div className="px-4 py-3 border-b border-gray-200 bg-gray-50">
          <h3 className="text-lg font-medium text-gray-900">{title}</h3>
        </div>
      )}
      <div className="max-h-96 overflow-y-auto">
        {items.length === 0 ? (
          <div className="py-5 px-4 text-center text-gray-600">
            No items available
          </div>
        ) : (
          <ul className="list-none p-0 m-0">
            {items.map((item, index) => (
              <li
                key={index}
                ref={index === highlightedIndex ? highlightedItemRef : null}
                className={`py-3 px-4 cursor-pointer border-b border-gray-200 hover:bg-gray-100 last:border-b-0 ${
                  index === highlightedIndex
                    ? "bg-yellow-100 border-2 border-yellow-400 hover:bg-yellow-100 text-yellow-900"
                    : ""
                }`}
                onClick={() => handleItemClick(item, index)}
                onMouseEnter={() => setHighlightedIndex(index)}
              >
                {item.displayName}
              </li>
            ))}
          </ul>
        )}
      </div>
    </Modal>
  );
};

export default ItemPicker;
