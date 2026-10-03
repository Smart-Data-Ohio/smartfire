// Licensed to the Software Freedom Conservancy (SFC) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The SFC licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.



bot.dom.trimExcludingNonBreakingSpaceCharacters_ = function (str) {
  return str.replace(/^[^\S\xa0]+|[^\S\xa0]+$/g, '');
};

bot.dom.concatenateCleanedLines_ = function (lines) {
  lines = goog.array.map(
    lines,
    bot.dom.trimExcludingNonBreakingSpaceCharacters_);
  var joined = lines.join('\n');
  var trimmed = bot.dom.trimExcludingNonBreakingSpaceCharacters_(joined);

  // Replace non-breakable spaces with regular ones.
  return trimmed.replace(/\xa0/g, ' ');
};

bot.dom.getVisibleText = function (elem) {
  var lines = [];

  if (bot.dom.IS_SHADOW_DOM_ENABLED) {
    bot.dom.appendVisibleTextLinesFromElementInComposedDom_(elem, lines);
  } else {
    bot.dom.appendVisibleTextLinesFromElement_(elem, lines);
  }
  return bot.dom.concatenateCleanedLines_(lines);
};

bot.dom.appendVisibleTextLinesFromElementCommon_ = function (
  elem, lines, isShownFn, childNodeFn) {
  function currLine() {
    return /** @type {string|undefined} */ (goog.array.peek(lines)) || '';
  }

  // TODO: Add case here for textual form elements.
  if (bot.dom.isElement(elem, goog.dom.TagName.BR)) {
    lines.push('');
  } else {
    // TODO: properly handle display:run-in
    var isTD = bot.dom.isElement(elem, goog.dom.TagName.TD);
    var display = bot.dom.getEffectiveStyle(elem, 'display');
    // On some browsers, table cells incorrectly show up with block styles.
    var isBlock = !isTD &&
      !goog.array.contains(bot.dom.INLINE_DISPLAY_BOXES_, display);

    // Add a newline before block elems when there is text on the current line,
    // except when the previous sibling has a display: run-in.
    // Also, do not run-in the previous sibling if this element is floated.

    var previousElementSibling = goog.dom.getPreviousElementSibling(elem);
    var prevDisplay = (previousElementSibling) ?
      bot.dom.getEffectiveStyle(previousElementSibling, 'display') : '';
    // TODO: getEffectiveStyle should mask this for us
    var thisFloat = bot.dom.getEffectiveStyle(elem, 'float') ||
      bot.dom.getEffectiveStyle(elem, 'cssFloat') ||
      bot.dom.getEffectiveStyle(elem, 'styleFloat');
    var runIntoThis = prevDisplay == 'run-in' && thisFloat == 'none';
    if (isBlock && !runIntoThis &&
      !goog.string.isEmptyOrWhitespace(currLine())) {
      lines.push('');
    }

    // This element may be considered unshown, but have a child that is
    // explicitly shown (e.g. this element has "visibility:hidden").
    // Nevertheless, any text nodes that are direct descendants of this
    // element will not contribute to the visible text.
    var shown = isShownFn(elem);

    // All text nodes that are children of this element need to know the
    // effective "white-space" and "text-transform" styles to properly
    // compute their contribution to visible text. Compute these values once.
    var whitespace = null, textTransform = null;
    if (shown) {
      whitespace = bot.dom.getEffectiveStyle(elem, 'white-space');
      textTransform = bot.dom.getEffectiveStyle(elem, 'text-transform');
    }

    goog.array.forEach(elem.childNodes, function (node) {
      childNodeFn(node, lines, shown, whitespace, textTransform);
    });

    var line = currLine();

    // Here we differ from standard innerText implementations (if there were
    // such a thing). Usually, table cells are separated by a tab, but we
    // normalize tabs into single spaces.
    if ((isTD || display == 'table-cell') && line &&
      !goog.string.endsWith(line, ' ')) {
      lines[lines.length - 1] += ' ';
    }

    // Add a newline after block elems when there is text on the current line,
    // and the current element isn't marked as run-in.
    if (isBlock && display != 'run-in' &&
      !goog.string.isEmptyOrWhitespace(line)) {
      lines.push('');
    }
  }
};

bot.dom.appendVisibleTextLinesFromElement_ = function (elem, lines) {
  bot.dom.appendVisibleTextLinesFromElementCommon_(
    elem, lines, bot.dom.isShown,
    function (node, lines, shown, whitespace, textTransform) {
      if (node.nodeType == goog.dom.NodeType.TEXT && shown) {
        var textNode = /** @type {!Text} */ (node);
        bot.dom.appendVisibleTextLinesFromTextNode_(textNode, lines,
          whitespace, textTransform);
      } else if (bot.dom.isElement(node)) {
        var castElem = /** @type {!Element} */ (node);
        bot.dom.appendVisibleTextLinesFromElement_(castElem, lines);
      }
    });
};

bot.dom.INLINE_DISPLAY_BOXES_ = [
  'inline',
  'inline-block',
  'inline-table',
  'none',
  'table-cell',
  'table-column',
  'table-column-group'
];

bot.dom.appendVisibleTextLinesFromTextNode_ = function (textNode, lines,
  whitespace, textTransform) {

  // First, remove zero-width characters. Do this before regularizing spaces as
  // the zero-width space is both zero-width and a space, but we do not want to
  // make it visible by converting it to a regular space.
  // The replaced characters are:
  //   U+200B: Zero-width space
  //   U+200E: Left-to-right mark
  //   U+200F: Right-to-left mark
  var text = textNode.nodeValue.replace(/[\u200b\u200e\u200f]/g, '');

  // Canonicalize the new lines, and then collapse new lines
  // for the whitespace styles that collapse. See:
  // https://developer.mozilla.org/en/CSS/white-space
  text = goog.string.canonicalizeNewlines(text);
  if (whitespace == 'normal' || whitespace == 'nowrap') {
    text = text.replace(/\n/g, ' ');
  }

  // For pre and pre-wrap whitespace styles, convert all breaking spaces to be
  // non-breaking, otherwise, collapse all breaking spaces. Breaking spaces are
  // converted to regular spaces by getVisibleText().
  if (whitespace == 'pre' || whitespace == 'pre-wrap') {
    text = text.replace(/[ \f\t\v\u2028\u2029]/g, '\xa0');
  } else {
    text = text.replace(/[\ \f\t\v\u2028\u2029]+/g, ' ');
  }

  if (textTransform == 'capitalize') {
    // the unicode regex ending with /gu does not work in IE
    var re = goog.userAgent.IE ? /(^|\s|\b)(\S)/g : /(^|\s|\b)(\S)/gu;
    text = text.replace(re, function () {
      return arguments[1] + arguments[2].toUpperCase();
    });
  } else if (textTransform == 'uppercase') {
    text = text.toUpperCase();
  } else if (textTransform == 'lowercase') {
    text = text.toLowerCase();
  }

  var currLine = lines.pop() || '';
  if (goog.string.endsWith(currLine, ' ') &&
    goog.string.startsWith(text, ' ')) {
    text = text.substr(1);
  }
  lines.push(currLine + text);
};

bot.dom.appendVisibleTextLinesFromNodeInComposedDom_ = function (
  node, lines, shown, whitespace, textTransform) {

  if (node.nodeType == goog.dom.NodeType.TEXT && shown) {
    var textNode = /** @type {!Text} */ (node);
    bot.dom.appendVisibleTextLinesFromTextNode_(textNode, lines,
      whitespace, textTransform);
  } else if (bot.dom.isElement(node)) {
    var castElem = /** @type {!Element} */ (node);

    if (bot.dom.isElement(node, 'CONTENT') || bot.dom.isElement(node, 'SLOT')) {
      var parentNode = node;
      while (parentNode.parentNode) {
        parentNode = parentNode.parentNode;
      }
      if (parentNode instanceof ShadowRoot) {
        // If the element is <content> and we're inside a shadow DOM then just
        // append the contents of the nodes that have been distributed into it.
        var contentElem = /** @type {!Object} */ (node);
        var shadowChildren;
        if (bot.dom.isElement(node, 'CONTENT')) {
          shadowChildren = contentElem.getDistributedNodes();
        } else {
          shadowChildren = contentElem.assignedNodes();
        }
        const childrenToTraverse =
          shadowChildren.length > 0 ? shadowChildren : contentElem.childNodes;
        goog.array.forEach(childrenToTraverse, function (node) {
          bot.dom.appendVisibleTextLinesFromNodeInComposedDom_(
            node, lines, shown, whitespace, textTransform);
        });
      } else {
        // if we're not inside a shadow DOM, then we just treat <content>
        // as an unknown element and use anything inside the tag
        bot.dom.appendVisibleTextLinesFromElementInComposedDom_(
          castElem, lines);
      }
    } else if (bot.dom.isElement(node, 'SHADOW')) {
      // if the element is <shadow> then find the owning shadowRoot
      var parentNode = node;
      while (parentNode.parentNode) {
        parentNode = parentNode.parentNode;
      }
      if (parentNode instanceof ShadowRoot) {
        var thisShadowRoot = /** @type {!ShadowRoot} */ (parentNode);
        if (thisShadowRoot) {
          // then go through the owning shadowRoots older siblings and append
          // their contents
          var olderShadowRoot = thisShadowRoot.olderShadowRoot;
          while (olderShadowRoot) {
            goog.array.forEach(
              olderShadowRoot.childNodes, function (childNode) {
                bot.dom.appendVisibleTextLinesFromNodeInComposedDom_(
                  childNode, lines, shown, whitespace, textTransform);
              });
            olderShadowRoot = olderShadowRoot.olderShadowRoot;
          }
        }
      }
    } else {
      // otherwise append the contents of an element as per normal.
      bot.dom.appendVisibleTextLinesFromElementInComposedDom_(
        castElem, lines);
    }
  }
};

bot.dom.isNodeDistributedIntoShadowDom = function (node) {
  var elemOrText = null;
  if (node.nodeType == goog.dom.NodeType.ELEMENT) {
    elemOrText = /** @type {!Element} */ (node);
  } else if (node.nodeType == goog.dom.NodeType.TEXT) {
    elemOrText = /** @type {!Text} */ (node);
  }
  return elemOrText != null &&
    (elemOrText.assignedSlot != null ||
      (elemOrText.getDestinationInsertionPoints &&
        elemOrText.getDestinationInsertionPoints().length > 0)
    );
};

bot.dom.appendVisibleTextLinesFromElementInComposedDom_ = function (
  elem, lines) {
  if (elem.shadowRoot) {
    goog.array.forEach(elem.shadowRoot.childNodes, function (node) {
      bot.dom.appendVisibleTextLinesFromNodeInComposedDom_(
        node, lines, true, null, null);
    });
  }

  bot.dom.appendVisibleTextLinesFromElementCommon_(
    elem, lines, bot.dom.isShown,
    function (node, lines, shown, whitespace, textTransform) {
      // If the node has been distributed into a shadowDom element
      // to be displayed elsewhere, then we shouldn't append
      // its contents here).
      if (!bot.dom.isNodeDistributedIntoShadowDom(node)) {
        bot.dom.appendVisibleTextLinesFromNodeInComposedDom_(
          node, lines, shown, whitespace, textTransform);
      }
    });
};
