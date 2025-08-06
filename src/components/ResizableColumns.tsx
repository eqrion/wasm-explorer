import * as React from "react";
import { useState, useRef, useCallback, useEffect } from "react";

export interface ColumnPanel {
    id: string;
    title: string;
    content: React.ReactNode;
    minWidth?: number;
    defaultWidth?: number;
    collapsible?: boolean;
}

interface ResizableColumnsProps {
    panels: ColumnPanel[];
    className?: string;
}

export function ResizableColumns({ panels, className = "" }: ResizableColumnsProps) {
    const [widths, setWidths] = useState<number[]>(() => 
        panels.map(panel => panel.defaultWidth || 100 / panels.length)
    );
    const [collapsed, setCollapsed] = useState<boolean[]>(() => 
        panels.map(() => false)
    );
    const [isResizing, setIsResizing] = useState<number | null>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const startXRef = useRef<number>(0);
    const startWidthsRef = useRef<number[]>([]);

    const handleMouseDown = useCallback((index: number, e: React.MouseEvent) => {
        e.preventDefault();
        setIsResizing(index);
        startXRef.current = e.clientX;
        startWidthsRef.current = [...widths];
    }, [widths]);

    const handleMouseMove = useCallback((e: MouseEvent) => {
        if (isResizing === null || !containerRef.current) return;

        const deltaX = e.clientX - startXRef.current;
        const containerWidth = containerRef.current.offsetWidth;
        const deltaPercent = (deltaX / containerWidth) * 100;

        const newWidths = [...startWidthsRef.current];
        const leftIndex = isResizing;
        const rightIndex = isResizing + 1;

        // Skip collapsed panels
        if (collapsed[leftIndex] || collapsed[rightIndex]) return;

        const minWidth = 10; // Minimum 10% width
        const leftMinWidth = panels[leftIndex]?.minWidth || minWidth;
        const rightMinWidth = panels[rightIndex]?.minWidth || minWidth;

        // Calculate new widths
        const newLeftWidth = Math.max(leftMinWidth, newWidths[leftIndex] + deltaPercent);
        const newRightWidth = Math.max(rightMinWidth, newWidths[rightIndex] - deltaPercent);

        // Only apply if both panels respect minimum widths
        if (newLeftWidth >= leftMinWidth && newRightWidth >= rightMinWidth) {
            newWidths[leftIndex] = newLeftWidth;
            newWidths[rightIndex] = newRightWidth;
            setWidths(newWidths);
        }
    }, [isResizing, collapsed, panels]);

    const handleMouseUp = useCallback(() => {
        setIsResizing(null);
    }, []);

    useEffect(() => {
        if (isResizing !== null) {
            document.addEventListener('mousemove', handleMouseMove);
            document.addEventListener('mouseup', handleMouseUp);
            return () => {
                document.removeEventListener('mousemove', handleMouseMove);
                document.removeEventListener('mouseup', handleMouseUp);
            };
        }
    }, [isResizing, handleMouseMove, handleMouseUp]);

    const toggleCollapse = useCallback((index: number) => {
        const newCollapsed = [...collapsed];
        newCollapsed[index] = !newCollapsed[index];
        setCollapsed(newCollapsed);

        // Redistribute width when collapsing/expanding
        const newWidths = [...widths];
        const visiblePanels = newCollapsed.filter(c => !c).length;
        
        if (newCollapsed[index]) {
            // Collapsing: distribute this panel's width to others
            const widthToDistribute = newWidths[index];
            newWidths[index] = 0;
            
            // Distribute to visible panels
            const otherVisibleIndices = newCollapsed.map((c, i) => !c && i !== index ? i : -1).filter(i => i !== -1);
            if (otherVisibleIndices.length > 0) {
                const extraWidth = widthToDistribute / otherVisibleIndices.length;
                otherVisibleIndices.forEach(i => {
                    newWidths[i] += extraWidth;
                });
            }
        } else {
            // Expanding: take proportional width from others
            const targetWidth = 100 / (visiblePanels + 1);
            const otherVisibleIndices = newCollapsed.map((c, i) => !c && i !== index ? i : -1).filter(i => i !== -1);
            
            const totalCurrentWidth = otherVisibleIndices.reduce((sum, i) => sum + newWidths[i], 0);
            const widthToTake = Math.min(targetWidth, totalCurrentWidth * 0.3); // Take at most 30% from others
            
            newWidths[index] = widthToTake;
            
            // Proportionally reduce other panels
            if (totalCurrentWidth > 0) {
                otherVisibleIndices.forEach(i => {
                    newWidths[i] = newWidths[i] * (100 - widthToTake) / totalCurrentWidth;
                });
            }
        }
        
        setWidths(newWidths);
    }, [collapsed, widths]);

    return (
        <div ref={containerRef} className={`flex h-full ${className}`}>
            {panels.map((panel, index) => (
                <React.Fragment key={panel.id}>
                    <div
                        className={`flex flex-col bg-white border-r border-gray-200 transition-all duration-200 ${
                            collapsed[index] ? 'min-w-0' : ''
                        }`}
                        style={{
                            width: collapsed[index] ? 'auto' : `${widths[index]}%`,
                            minWidth: collapsed[index] ? 'auto' : `${panels[index]?.minWidth || 10}%`
                        }}
                    >
                        {/* Panel Header */}
                        <div className="flex items-center justify-between px-3 py-2 bg-gray-50 border-b border-gray-200 min-w-0">
                            <span className={`font-medium text-sm text-gray-700 ${collapsed[index] ? 'sr-only' : 'truncate'}`}>
                                {panel.title}
                            </span>
                            {panel.collapsible !== false && (
                                <button
                                    onClick={() => toggleCollapse(index)}
                                    className="p-1 text-gray-500 hover:text-gray-700 hover:bg-gray-200 rounded transition-colors flex-shrink-0"
                                    title={collapsed[index] ? `Expand ${panel.title}` : `Collapse ${panel.title}`}
                                >
                                    {collapsed[index] ? (
                                        <svg className="w-4 h-4 transform rotate-90" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                                        </svg>
                                    ) : (
                                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                                        </svg>
                                    )}
                                </button>
                            )}
                        </div>

                        {/* Panel Content */}
                        {!collapsed[index] && (
                            <div className="flex-1 overflow-auto">
                                {panel.content}
                            </div>
                        )}

                        {/* Collapsed Panel Indicator */}
                        {collapsed[index] && (
                            <div className="flex-1 flex items-center justify-center p-2 min-w-0">
                                <div 
                                    className="writing-mode-vertical text-xs text-gray-500 transform rotate-180 whitespace-nowrap cursor-pointer hover:text-gray-700"
                                    onClick={() => toggleCollapse(index)}
                                    title={`Expand ${panel.title}`}
                                    style={{ writingMode: 'vertical-rl' }}
                                >
                                    {panel.title}
                                </div>
                            </div>
                        )}
                    </div>

                    {/* Resize Handle */}
                    {index < panels.length - 1 && !collapsed[index] && !collapsed[index + 1] && (
                        <div
                            className={`w-1 bg-gray-200 hover:bg-blue-400 cursor-col-resize flex-shrink-0 transition-colors ${
                                isResizing === index ? 'bg-blue-500' : ''
                            }`}
                            onMouseDown={(e) => handleMouseDown(index, e)}
                        />
                    )}
                </React.Fragment>
            ))}
        </div>
    );
}
