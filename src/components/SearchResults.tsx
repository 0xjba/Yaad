import React, { useState } from "react";
import { SearchResult } from "../types";

interface SearchResultsProps {
  results: SearchResult[];
}

const SearchResults: React.FC<SearchResultsProps> = ({ results }) => {
  const [expandedId, setExpandedId] = useState<string | null>(
    results.length > 0 ? results[0].memory.id : null
  );

  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    return date.toLocaleDateString("en-US", {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  };

  return (
    <div className="space-y-3 max-h-96 overflow-y-auto">
      {results.map((result, index) => {
        const isExpanded = expandedId === result.memory.id;
        const isTop = index === 0;

        return (
          <div
            key={result.memory.id}
            className={`bg-white border border-gray-200 rounded-lg p-4 transition-all duration-300 cursor-pointer ${
              isExpanded 
                ? "shadow-lg border-blue-300 scale-[1.02] ring-2 ring-blue-100" 
                : "shadow-sm hover:shadow-md hover:border-gray-300 hover:scale-[1.01]"
            }`}
            onClick={() => setExpandedId(isExpanded ? null : result.memory.id)}
          >
            <div className="flex items-start justify-between">
              <div className="flex-1">
                <div className="flex items-center gap-2 mb-2">
                  <span className="text-xs text-gray-500">
                    {formatDate(result.memory.created_at)}
                  </span>
                  <span className="text-xs text-blue-600">
                    {(result.similarity * 100).toFixed(1)}% match
                  </span>
                </div>
                <p
                  className={`text-gray-800 transition-all ${
                    isExpanded ? "line-clamp-none" : "line-clamp-2"
                  }`}
                >
                  {result.memory.content}
                </p>
                {isExpanded && (
                  <div className="mt-3 pt-3 border-t border-gray-200">
                    {result.memory.context_url && (
                      <a
                        href={result.memory.context_url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-blue-600 hover:underline text-sm"
                        onClick={(e) => e.stopPropagation()}
                      >
                        🔗 {result.memory.context_url}
                      </a>
                    )}
                    {result.memory.context_note && (
                      <p className="text-sm text-gray-600 mt-2">
                        {result.memory.context_note}
                      </p>
                    )}
                    {result.memory.duration_sec && (
                      <p className="text-xs text-gray-500 mt-2">
                        Duration: {result.memory.duration_sec}s
                      </p>
                    )}
                  </div>
                )}
              </div>
              <button
                className="ml-2 text-gray-400 hover:text-gray-600 transition-colors"
                onClick={(e) => {
                  e.stopPropagation();
                  setExpandedId(isExpanded ? null : result.memory.id);
                }}
              >
                <svg
                  className={`w-5 h-5 transition-transform ${
                    isExpanded ? "rotate-180" : ""
                  }`}
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M19 9l-7 7-7-7"
                  />
                </svg>
              </button>
            </div>
          </div>
        );
      })}
    </div>
  );
};

export default SearchResults;

