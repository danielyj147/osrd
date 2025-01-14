import React from 'react';

import cx from 'classnames';
import { useTranslation } from 'react-i18next';

import InputSNCF from 'common/BootstrapSNCF/InputSNCF';
import OptionsSNCF from 'common/BootstrapSNCF/OptionsSNCF';

import type { ValidityFilter } from './types';

type FilterPanelProps = {
  filter: string;
  setFilter: (filter: string) => void;
  rollingStockFilter: string;
  setRollingStockFilter: (rollingStockFilter: string) => void;
  validityFilter: ValidityFilter;
  setValidityFilter: (validityFilter: ValidityFilter) => void;
  uniqueTags: string[];
  selectedTags: Set<string | null>;
  setSelectedTags: React.Dispatch<React.SetStateAction<Set<string | null>>>;
};

const FilterPanel = ({
  filter,
  setFilter,
  rollingStockFilter,
  setRollingStockFilter,
  validityFilter,
  setValidityFilter,
  uniqueTags,
  selectedTags,
  setSelectedTags,
}: FilterPanelProps) => {
  const { t } = useTranslation('operationalStudies/scenario');

  const validityOptions: { value: ValidityFilter; label: string }[] = [
    { value: 'both', label: t('timetable.showAllTrains') },
    { value: 'valid', label: t('timetable.showValidTrains') },
    { value: 'invalid', label: t('timetable.showInvalidTrains') },
  ];

  const toggleTagSelection = (tag: string | null) => {
    setSelectedTags((prevSelectedTags) => {
      const newSelectedTags = new Set(prevSelectedTags);
      if (newSelectedTags.has(tag)) {
        newSelectedTags.delete(tag);
      } else {
        newSelectedTags.add(tag);
      }
      return newSelectedTags;
    });
  };

  return (
    <div className="filter-panel">
      <div className="row">
        <div className="col-5">
          <InputSNCF
            type="text"
            id="timetable-label-filter"
            name="timetable-label-filter"
            label={t('timetable.filterLabel')}
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder={t('filterPlaceholder')}
            noMargin
            unit={<i className="icons-search" />}
            sm
            data-testid="timetable-label-filter"
            title={t('filterPlaceholder')}
          />
          <div className="my-3" />
          <InputSNCF
            type="text"
            id="timetable-rollingstock-filter"
            name="timetable-rollingstock-filter"
            label={t('timetable.advancedFilterLabel')}
            value={rollingStockFilter}
            onChange={(e) => setRollingStockFilter(e.target.value)}
            placeholder={t('timetable.rollingStockFilterPlaceholder')}
            noMargin
            unit={<i className="icons-search" />}
            sm
            data-testid="timetable-rollingstock-filter"
            title={t('timetable.rollingStockFilterPlaceholder')}
          />
        </div>

        <div className="col-7">
          <label htmlFor="train-validity">{t('timetable.validityFilter')}</label>
          <div className="validity-filter">
            <OptionsSNCF
              onChange={(event) => setValidityFilter(event.target.value as ValidityFilter)}
              options={validityOptions}
              name="train-validity"
              selectedValue={validityFilter}
            />
          </div>

          <label htmlFor="composition-tag-filter">{t('timetable.compositionCodes')}</label>
          <div className="composition-tag-filter" id="composition-tag-filter">
            {uniqueTags.map((tag) => {
              const displayTag = tag !== 'NO CODE' ? tag : t('timetable.noSpeedLimitTags');
              return (
                <button
                  className={cx('btn', 'btn-sm', { selectedTag: selectedTags.has(tag) })}
                  type="button"
                  key={tag}
                  onClick={() => toggleTagSelection(tag)}
                >
                  {displayTag}
                </button>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
};

export default FilterPanel;
