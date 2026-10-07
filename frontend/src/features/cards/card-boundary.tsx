import { Component, type ReactNode } from "react";

interface CardBoundaryProps {
  readonly children: ReactNode;
}

interface CardBoundaryState {
  readonly failed: boolean;
}

/**
 * Keeps one card's failure to itself: a card whose data this build can't read (the store keeps
 * the wire JSON, which the tolerant schema let through) renders nothing instead of taking the
 * timeline down, as the contract asks of unreadable cards.
 */
export class CardBoundary extends Component<CardBoundaryProps, CardBoundaryState> {
  override state: CardBoundaryState = { failed: false };

  static getDerivedStateFromError(): CardBoundaryState {
    return { failed: true };
  }

  override render(): ReactNode {
    return this.state.failed ? null : this.props.children;
  }
}
